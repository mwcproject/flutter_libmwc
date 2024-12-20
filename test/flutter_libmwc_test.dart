import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_libmwc/flutter_libmwc.dart';
import 'package:flutter_libmwc/flutter_libmwc_platform_interface.dart';
import 'package:flutter_libmwc/flutter_libmwc_method_channel.dart';
import 'package:plugin_platform_interface/plugin_platform_interface.dart';

class MockFlutterLibmwcPlatform
    with MockPlatformInterfaceMixin
    implements FlutterLibmwcPlatform {

  @override
  Future<String?> getPlatformVersion() => Future.value('42');
}

void main() {
  final FlutterLibmwcPlatform initialPlatform = FlutterLibmwcPlatform.instance;

  test('$MethodChannelFlutterLibmwc is the default instance', () {
    expect(initialPlatform, isInstanceOf<MethodChannelFlutterLibmwc>());
  });

  test('getPlatformVersion', () async {
    FlutterLibmwc flutterLibmwcPlugin = FlutterLibmwc();
    MockFlutterLibmwcPlatform fakePlatform = MockFlutterLibmwcPlatform();
    FlutterLibmwcPlatform.instance = fakePlatform;

    expect(await flutterLibmwcPlugin.getPlatformVersion(), '42');
  });
}
