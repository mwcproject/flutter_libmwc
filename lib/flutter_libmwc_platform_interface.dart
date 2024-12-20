import 'package:plugin_platform_interface/plugin_platform_interface.dart';

import 'flutter_libmwc_method_channel.dart';

abstract class FlutterLibmwcPlatform extends PlatformInterface {
  /// Constructs a FlutterLibmwcPlatform.
  FlutterLibmwcPlatform() : super(token: _token);

  static final Object _token = Object();

  static FlutterLibmwcPlatform _instance = MethodChannelFlutterLibmwc();

  /// The default instance of [FlutterLibmwcPlatform] to use.
  ///
  /// Defaults to [MethodChannelFlutterLibmwc].
  static FlutterLibmwcPlatform get instance => _instance;

  /// Platform-specific implementations should set this with their own
  /// platform-specific class that extends [FlutterLibmwcPlatform] when
  /// they register themselves.
  static set instance(FlutterLibmwcPlatform instance) {
    PlatformInterface.verifyToken(instance, _token);
    _instance = instance;
  }

  Future<String?> getPlatformVersion() {
    throw UnimplementedError('platformVersion() has not been implemented.');
  }
}
