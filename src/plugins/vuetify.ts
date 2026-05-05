import { createVuetify } from "vuetify";
import { aliases, mdi } from "vuetify/iconsets/mdi";

const jukeboyTheme = {
  dark: true,
  colors: {
    background: "#121212",
    surface: "#1e1e1e",
    "surface-bright": "#262626",
    "surface-light": "#2c2c2c",
    "surface-variant": "#2c2c2c",
    primary: "#ffffff",
    secondary: "#a0a0a0",
    accent: "#f3f3f3",
    error: "#f5f5f5",
    info: "#d8d8d8",
    success: "#ffffff",
    warning: "#e6e6e6",
    "on-background": "#ffffff",
    "on-surface": "#ffffff",
    "on-surface-variant": "#d4d4d4",
    "on-primary": "#000000",
  },
};

export default createVuetify({
  display: {
    mobileBreakpoint: "md",
  },
  icons: {
    defaultSet: "mdi",
    aliases,
    sets: { mdi },
  },
  theme: {
    defaultTheme: "jukeboyTheme",
    themes: {
      jukeboyTheme,
    },
  },
  defaults: {
    VAppBar: {
      elevation: 0,
      flat: true,
      color: "transparent",
    },
    VBtn: {
      rounded: "pill",
      height: 56,
    },
    VCard: {
      rounded: "xl",
      elevation: 0,
    },
    VSheet: {
      rounded: "xl",
      elevation: 0,
    },
    VTextField: {
      variant: "solo-filled",
      flat: true,
      density: "comfortable",
      hideDetails: "auto",
    },
    VSelect: {
      variant: "solo-filled",
      flat: true,
      density: "comfortable",
      hideDetails: "auto",
    },
    VTextarea: {
      variant: "solo-filled",
      flat: true,
      density: "comfortable",
      hideDetails: "auto",
    },
    VSwitch: {
      color: "primary",
      hideDetails: true,
      inset: true,
    },
    VSlider: {
      color: "primary",
      trackColor: "surface-variant",
      thumbSize: 30,
      trackSize: 10,
      hideDetails: true,
    },
    VList: {
      bgColor: "transparent",
    },
    VListItem: {
      rounded: "xl",
      minHeight: 72,
    },
    VNavigationDrawer: {
      color: "surface",
      elevation: 0,
    },
    VBottomNavigation: {
      bgColor: "surface",
      elevation: 0,
      grow: true,
    },
  },
});