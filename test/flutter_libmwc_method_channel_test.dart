import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_libmwc/flutter_libmwc_method_channel.dart';

void main() {
  MethodChannelFlutterLibmwc platform = MethodChannelFlutterLibmwc();
  const MethodChannel channel = MethodChannel('flutter_libmwc');

  TestWidgetsFlutterBinding.ensureInitialized();

  setUp(() {
    channel.setMockMethodCallHandler((MethodCall methodCall) async {
      return '42';
    });
  });

  tearDown(() {
    channel.setMockMethodCallHandler(null);
  });

  test('getPlatformVersion', () async {
    expect(await platform.getPlatformVersion(), '42');
  });
}
