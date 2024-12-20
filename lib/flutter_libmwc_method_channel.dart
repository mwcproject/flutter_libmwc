import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import 'flutter_libmwc_platform_interface.dart';

/// An implementation of [FlutterLibmwcPlatform] that uses method channels.
class MethodChannelFlutterLibmwc extends FlutterLibmwcPlatform {
  /// The method channel used to interact with the native platform.
  @visibleForTesting
  final methodChannel = const MethodChannel('flutter_libmwc');

  @override
  Future<String?> getPlatformVersion() async {
    final version = await methodChannel.invokeMethod<String>('getPlatformVersion');
    return version;
  }
}
