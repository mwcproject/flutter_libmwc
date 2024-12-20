#include "include/flutter_libmwc/flutter_libmwc_plugin_c_api.h"

#include <flutter/plugin_registrar_windows.h>

#include "flutter_libmwc_plugin.h"

void FlutterLibmwcPluginCApiRegisterWithRegistrar(
    FlutterDesktopPluginRegistrarRef registrar) {
  flutter_libmwc::FlutterLibmwcPlugin::RegisterWithRegistrar(
      flutter::PluginRegistrarManager::GetInstance()
          ->GetRegistrar<flutter::PluginRegistrarWindows>(registrar));
}
