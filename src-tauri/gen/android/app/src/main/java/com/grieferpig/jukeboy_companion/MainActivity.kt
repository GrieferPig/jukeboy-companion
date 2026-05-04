package com.grieferpig.jukeboy_companion

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import androidx.activity.enableEdgeToEdge
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    requestBluetoothPermissionsIfNeeded()
  }

  private fun requestBluetoothPermissionsIfNeeded() {
    val missingPermissions = requiredBluetoothPermissions().filter { permission ->
      ContextCompat.checkSelfPermission(this, permission) != PackageManager.PERMISSION_GRANTED
    }

    if (missingPermissions.isNotEmpty()) {
      ActivityCompat.requestPermissions(this, missingPermissions.toTypedArray(), REQUEST_BLUETOOTH_PERMISSIONS)
    }
  }

  private fun requiredBluetoothPermissions(): Array<String> {
    return when {
      Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> arrayOf(
        Manifest.permission.BLUETOOTH_SCAN,
        Manifest.permission.BLUETOOTH_CONNECT,
      )
      Build.VERSION.SDK_INT >= Build.VERSION_CODES.M -> arrayOf(Manifest.permission.ACCESS_FINE_LOCATION)
      else -> emptyArray()
    }
  }

  companion object {
    private const val REQUEST_BLUETOOTH_PERMISSIONS = 1001
  }
}
