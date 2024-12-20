
import 'flutter_libmwc_platform_interface.dart';

class FlutterLibmwc {
  Future<String?> getPlatformVersion() {
    return FlutterLibmwcPlatform.instance.getPlatformVersion();
  }
}
