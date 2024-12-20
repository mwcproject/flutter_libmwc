Pod::Spec.new do |s|
  s.name             = 'flutter_libmwc'
  s.version          = '0.0.1'
  s.summary          = 'Binaries required to use flutter_libmwc in a Flutter project'
  s.description      = <<-DESC
Binaries required to use flutter_libmwc in a Flutter project. This library provides MWC wallet integration for Flutter apps.
  DESC
  s.homepage         = 'https://github.com/mwcproject/flutter_libmwc'
  s.license          = { :type => 'MIT', :file => 'LICENSE' }
  s.author           = { 'MWC Team' => 'info@mwc.mw' }

  # Point to the GitHub repository and tag for the source
  s.source           = { :git => 'https://github.com/mwcproject/flutter_libmwc.git', :tag => s.version.to_s }

  # Specify the source files and framework dependencies
  s.source_files     = 'Classes/**/*'
  s.vendored_frameworks = 'Frameworks/MwcWallet.framework'

  # Add Flutter dependency for macOS
  s.dependency       'FlutterMacOS'

  # macOS deployment target
  s.platform         = :osx, '10.13'

  # Configure Swift support
  s.pod_target_xcconfig = { 'DEFINES_MODULE' => 'YES' }
  s.swift_version    = '5.0'
end
