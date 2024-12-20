//
//  Generated file. Do not edit.
//

// clang-format off

#include "generated_plugin_registrant.h"

#include <flutter_libmwc/flutter_libmwc_plugin_c_api.h>
#include <flutter_secure_storage_windows/flutter_secure_storage_windows_plugin.h>

void RegisterPlugins(flutter::PluginRegistry* registry) {
  FlutterLibmwcPluginCApiRegisterWithRegistrar(
      registry->GetRegistrarForPlugin("FlutterLibmwcPluginCApi"));
  FlutterSecureStorageWindowsPluginRegisterWithRegistrar(
      registry->GetRegistrarForPlugin("FlutterSecureStorageWindowsPlugin"));
}
