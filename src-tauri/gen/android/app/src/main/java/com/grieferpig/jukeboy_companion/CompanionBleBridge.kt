package com.grieferpig.jukeboy_companion

import android.annotation.SuppressLint
import android.app.Activity
import android.Manifest
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCallback
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattDescriptor
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.bluetooth.BluetoothStatusCodes
import android.bluetooth.le.ScanCallback
import android.bluetooth.le.ScanResult
import android.bluetooth.le.ScanSettings
import android.os.Build
import android.os.Handler
import android.os.Looper
import androidx.core.content.ContextCompat
import org.json.JSONArray
import org.json.JSONObject
import java.util.Locale
import java.util.UUID
import java.util.concurrent.CompletableFuture
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.TimeoutException

class CompanionBleBridge private constructor(private val activity: Activity) {
  private val mainHandler = Handler(Looper.getMainLooper())
  private val sessions = ConcurrentHashMap<Long, GattSession>()

  private external fun nativeOnNotification(sessionId: Long, payload: ByteArray)
  private external fun nativeOnSessionClosed(sessionId: Long, reason: String)

  fun scanDevices(serviceUuid: String, timeoutMs: Long): String {
    ensureScanPermissionGranted()

    val bluetoothAdapter = requireBluetoothAdapter()
    val scanner = bluetoothAdapter.bluetoothLeScanner
      ?: throw IllegalStateException("Bluetooth LE scanner is unavailable")

    val devices = ConcurrentHashMap<String, ScanDevice>()
    val completed = CountDownLatch(1)
    val serviceUuidLower = serviceUuid.lowercase(Locale.US)
    var scanFailure: Throwable? = null

    val callback = object : ScanCallback() {
      override fun onScanResult(callbackType: Int, result: ScanResult) {
        val device = result.device ?: return
        val address = device.address ?: return
        val uuids = result.scanRecord?.serviceUuids
          ?.map { uuid -> uuid.uuid.toString().lowercase(Locale.US) }
          ?: emptyList()
        val name = result.scanRecord?.deviceName ?: readDeviceName(device) ?: ""

        devices[address] = ScanDevice(
          address = address,
          name = name,
          serviceMatch = uuids.any { uuid -> uuid == serviceUuidLower },
          uuids = uuids,
        )
      }

      override fun onScanFailed(errorCode: Int) {
        scanFailure = IllegalStateException("BLE scan failed with error $errorCode")
        completed.countDown()
      }
    }

    val timeout = timeoutMs.coerceAtLeast(1)
    val stopScan = Runnable {
      runCatching { scanner.stopScan(callback) }
      completed.countDown()
    }

    try {
      scanner.startScan(
        null,
        ScanSettings.Builder()
          .setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY)
          .build(),
        callback,
      )
      mainHandler.postDelayed(stopScan, timeout)

      if (!completed.await(timeout + 1000, TimeUnit.MILLISECONDS)) {
        throw TimeoutException("BLE scan timed out")
      }
      scanFailure?.let { throw it }
    } finally {
      mainHandler.removeCallbacks(stopScan)
      runCatching { scanner.stopScan(callback) }
    }

    return JSONArray().apply {
      devices.values.sortedBy { device -> device.address }.forEach { device ->
        put(device.toJson())
      }
    }.toString()
  }

  fun connectSession(
    sessionId: Long,
    address: String,
    serviceUuid: String,
    writeUuid: String,
    notifyUuid: String,
    timeoutMs: Long,
  ): String {
    ensureConnectPermissionGranted()

    val device = requireBluetoothAdapter().getRemoteDevice(address)
    val session = GattSession(
      sessionId = sessionId,
      device = device,
      serviceUuid = UUID.fromString(serviceUuid),
      writeUuid = UUID.fromString(writeUuid),
      notifyUuid = UUID.fromString(notifyUuid),
    )
    sessions[sessionId] = session

    return try {
      session.connect(timeoutMs).toString()
    } catch (error: Throwable) {
      sessions.remove(sessionId)
      session.close(error.message ?: "BLE connect failed")
      throw error
    }
  }

  fun writeChunk(sessionId: Long, payload: ByteArray) {
    val session = sessions[sessionId]
      ?: throw IllegalStateException("BLE session $sessionId is not available")
    session.writeChunk(payload)
  }

  fun disconnectSession(sessionId: Long) {
    ensureConnectPermissionGranted()

    val session = sessions.remove(sessionId) ?: return
    session.close("BLE session disconnected")
  }

  private fun ensureScanPermissionGranted() {
    when {
      Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> ensurePermissionGranted(
        Manifest.permission.BLUETOOTH_SCAN,
        "Android Bluetooth scan permission has not been granted yet",
      )
      Build.VERSION.SDK_INT >= Build.VERSION_CODES.M -> ensurePermissionGranted(
        Manifest.permission.ACCESS_FINE_LOCATION,
        "Android location permission has not been granted yet",
      )
    }
  }

  private fun ensureConnectPermissionGranted() {
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
      ensurePermissionGranted(
        Manifest.permission.BLUETOOTH_CONNECT,
        "Android Bluetooth connect permission has not been granted yet",
      )
    }
  }

  private fun ensurePermissionGranted(permission: String, message: String) {
    if (ContextCompat.checkSelfPermission(activity, permission) != android.content.pm.PackageManager.PERMISSION_GRANTED) {
      throw IllegalStateException(message)
    }
  }

  private fun requireBluetoothAdapter(): BluetoothAdapter {
    return activity.getSystemService(BluetoothManager::class.java)?.adapter
      ?: throw IllegalStateException("Bluetooth adapter is unavailable")
  }

  @SuppressLint("MissingPermission")
  private fun readDeviceName(device: BluetoothDevice): String? {
    return runCatching { device.name }.getOrNull()
  }

  private fun onSessionClosed(sessionId: Long, reason: String) {
    sessions.remove(sessionId)
    nativeOnSessionClosed(sessionId, reason)
  }

  private inner class GattSession(
    private val sessionId: Long,
    private val device: BluetoothDevice,
    private val serviceUuid: UUID,
    private val writeUuid: UUID,
    private val notifyUuid: UUID,
  ) {
    private var gatt: BluetoothGatt? = null
    private var writeCharacteristic: BluetoothGattCharacteristic? = null
    private var notifyCharacteristic: BluetoothGattCharacteristic? = null

    @Volatile
    private var connectFuture = CompletableFuture<Unit>()

    @Volatile
    private var discoverFuture = CompletableFuture<Unit>()

    @Volatile
    private var descriptorFuture = CompletableFuture<Unit>()

    @Volatile
    private var writeFuture = CompletableFuture<Unit>()

    private val callback = object : BluetoothGattCallback() {
      override fun onConnectionStateChange(gatt: BluetoothGatt, status: Int, newState: Int) {
        if (status != BluetoothGatt.GATT_SUCCESS) {
          fail("connection state change failed with status $status")
          return
        }

        when (newState) {
          BluetoothProfile.STATE_CONNECTED -> connectFuture.complete(Unit)
          BluetoothProfile.STATE_DISCONNECTED -> fail("BLE device disconnected")
        }
      }

      override fun onServicesDiscovered(gatt: BluetoothGatt, status: Int) {
        if (status == BluetoothGatt.GATT_SUCCESS) {
          discoverFuture.complete(Unit)
        } else {
          fail("service discovery failed with status $status")
        }
      }

      override fun onDescriptorWrite(
        gatt: BluetoothGatt,
        descriptor: BluetoothGattDescriptor,
        status: Int,
      ) {
        if (status == BluetoothGatt.GATT_SUCCESS) {
          descriptorFuture.complete(Unit)
        } else {
          fail("notification setup failed with status $status")
        }
      }

      override fun onCharacteristicWrite(
        gatt: BluetoothGatt,
        characteristic: BluetoothGattCharacteristic,
        status: Int,
      ) {
        if (status == BluetoothGatt.GATT_SUCCESS) {
          writeFuture.complete(Unit)
        } else {
          fail("characteristic write failed with status $status")
        }
      }

      @Suppress("DEPRECATION", "OVERRIDE_DEPRECATION")
      override fun onCharacteristicChanged(
        gatt: BluetoothGatt,
        characteristic: BluetoothGattCharacteristic,
      ) {
        nativeOnNotification(sessionId, characteristic.value ?: ByteArray(0))
      }

      override fun onCharacteristicChanged(
        gatt: BluetoothGatt,
        characteristic: BluetoothGattCharacteristic,
        value: ByteArray,
      ) {
        nativeOnNotification(sessionId, value)
      }
    }

    @SuppressLint("MissingPermission")
    fun connect(timeoutMs: Long): JSONObject {
      val timeout = timeoutMs.coerceAtLeast(1000)
      connectFuture = CompletableFuture()
      discoverFuture = CompletableFuture()
      descriptorFuture = CompletableFuture()
      writeFuture = CompletableFuture()

      gatt = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
        device.connectGatt(activity, false, callback, BluetoothDevice.TRANSPORT_LE)
      } else {
        @Suppress("DEPRECATION")
        device.connectGatt(activity, false, callback)
      }

      val bluetoothGatt = gatt ?: throw IllegalStateException("failed to open BLE GATT session")
      await(connectFuture, timeout, "connect")

      if (!bluetoothGatt.discoverServices()) {
        throw IllegalStateException("failed to start BLE service discovery")
      }
      await(discoverFuture, timeout, "service discovery")

      val service = bluetoothGatt.getService(serviceUuid)
        ?: throw IllegalStateException("BLE service $serviceUuid was not found")
      writeCharacteristic = service.getCharacteristic(writeUuid)
        ?: throw IllegalStateException("BLE write characteristic $writeUuid was not found")
      notifyCharacteristic = service.getCharacteristic(notifyUuid)
        ?: throw IllegalStateException("BLE notify characteristic $notifyUuid was not found")

      enableNotifications(timeout)

      return JSONObject().apply {
        put("name", readDeviceName(device) ?: device.address ?: "")
      }
    }

    @SuppressLint("MissingPermission")
    fun writeChunk(payload: ByteArray) {
      val bluetoothGatt = gatt ?: throw IllegalStateException("BLE session is not connected")
      val writeCharacteristic = writeCharacteristic
        ?: throw IllegalStateException("BLE write characteristic is unavailable")

      writeFuture = CompletableFuture()
      if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
        val status = bluetoothGatt.writeCharacteristic(
          writeCharacteristic,
          payload,
          BluetoothGattCharacteristic.WRITE_TYPE_NO_RESPONSE,
        )
        if (status != BluetoothStatusCodes.SUCCESS) {
          throw IllegalStateException("failed to queue BLE write: status $status")
        }
      } else {
        @Suppress("DEPRECATION")
        writeCharacteristic.writeType = BluetoothGattCharacteristic.WRITE_TYPE_NO_RESPONSE
        @Suppress("DEPRECATION")
        writeCharacteristic.value = payload
        @Suppress("DEPRECATION")
        if (!bluetoothGatt.writeCharacteristic(writeCharacteristic)) {
          throw IllegalStateException("failed to queue BLE write")
        }
      }

      await(writeFuture, 5000, "write")
    }

    @SuppressLint("MissingPermission")
    fun close(reason: String) {
      completeIfPending(connectFuture, reason)
      completeIfPending(discoverFuture, reason)
      completeIfPending(descriptorFuture, reason)
      completeIfPending(writeFuture, reason)

      val bluetoothGatt = gatt
      gatt = null
      runCatching {
        bluetoothGatt?.disconnect()
      }
      runCatching {
        bluetoothGatt?.close()
      }

      onSessionClosed(sessionId, reason)
    }

    @SuppressLint("MissingPermission")
    private fun enableNotifications(timeoutMs: Long) {
      val bluetoothGatt = gatt ?: throw IllegalStateException("BLE session is not connected")
      val notifyCharacteristic = notifyCharacteristic
        ?: throw IllegalStateException("BLE notify characteristic is unavailable")

      if (!bluetoothGatt.setCharacteristicNotification(notifyCharacteristic, true)) {
        throw IllegalStateException("failed to enable BLE notifications")
      }

      val descriptor = notifyCharacteristic.getDescriptor(CLIENT_CHARACTERISTIC_CONFIGURATION_UUID)
        ?: throw IllegalStateException("BLE notify descriptor is unavailable")
      descriptorFuture = CompletableFuture()

      if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
        val status = bluetoothGatt.writeDescriptor(
          descriptor,
          BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE,
        )
        if (status != BluetoothStatusCodes.SUCCESS) {
          throw IllegalStateException("failed to write BLE notify descriptor: status $status")
        }
      } else {
        @Suppress("DEPRECATION")
        descriptor.value = BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE
        @Suppress("DEPRECATION")
        if (!bluetoothGatt.writeDescriptor(descriptor)) {
          throw IllegalStateException("failed to write BLE notify descriptor")
        }
      }

      await(descriptorFuture, timeoutMs, "enable notifications")
    }

    private fun fail(message: String) {
      completeIfPending(connectFuture, message)
      completeIfPending(discoverFuture, message)
      completeIfPending(descriptorFuture, message)
      completeIfPending(writeFuture, message)
      close(message)
    }

    private fun await(future: CompletableFuture<Unit>, timeoutMs: Long, step: String) {
      try {
        future.get(timeoutMs.coerceAtLeast(1), TimeUnit.MILLISECONDS)
      } catch (error: TimeoutException) {
        throw IllegalStateException("BLE $step timed out", error)
      } catch (error: Exception) {
        throw IllegalStateException("BLE $step failed", error)
      }
    }

    private fun completeIfPending(future: CompletableFuture<Unit>, message: String) {
      if (!future.isDone) {
        future.completeExceptionally(IllegalStateException(message))
      }
    }
  }

  private data class ScanDevice(
    val address: String,
    val name: String,
    val serviceMatch: Boolean,
    val uuids: List<String>,
  ) {
    fun toJson(): JSONObject {
      return JSONObject().apply {
        put("address", address)
        put("name", name)
        put("service_match", serviceMatch)
        put("uuids", JSONArray().apply {
          uuids.forEach { uuid -> put(uuid) }
        })
      }
    }
  }

  companion object {
    private val CLIENT_CHARACTERISTIC_CONFIGURATION_UUID: UUID =
      UUID.fromString("00002902-0000-1000-8000-00805f9b34fb")

    @Volatile
    private var instance: CompanionBleBridge? = null

    @JvmStatic
    fun getInstance(activity: Activity): CompanionBleBridge {
      return instance ?: synchronized(this) {
        instance ?: CompanionBleBridge(activity).also { bridge ->
          instance = bridge
        }
      }
    }
  }
}