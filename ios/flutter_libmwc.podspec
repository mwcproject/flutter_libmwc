#
# To learn more about a Podspec see http://guides.cocoapods.org/syntax/podspec.html.
# Run `pod lib lint flutter_libmwc.podspec` to validate before publishing.
#
Pod::Spec.new do |s|
  s.name             = 'flutter_libmwc'
  s.version          = '0.0.3'
  s.summary          = 'Binaries required to use flutter_libmwc in a Flutter project'
  s.description      = <<-DESC
Binaries required to use flutter_libmwc in a Flutter project. This library provides MWC wallet integration for Flutter apps, enabling wallet management, transactions, and more.
  DESC
  s.homepage         = 'https://github.com/mwcproject/flutter_libmwc'
  s.license          = { :type => 'MIT', :file => 'LICENSE' }
  s.author           = { 'MWC Team' => 'info@mwc.mw' }

  # Point to the GitHub repository and tag for the source
  s.source = { :git => 'https://github.com/mwcproject/flutter_libmwc.git', :tag => s.version.to_s }

  # Specify the source files and framework dependencies
  s.source_files     = 'ios/Classes/**/*'
  s.vendored_frameworks = 'ios/Frameworks/MwcWallet.framework'

  # Add Flutter dependency
  s.dependency       'Flutter'

  # iOS deployment target
  s.platform         = :ios, '12.0'

  # Configure architecture exclusions and Swift support
  s.pod_target_xcconfig = {
    'DEFINES_MODULE' => 'YES',
    'EXCLUDED_ARCHS[sdk=iphonesimulator*]' => 'i386'
  }
  s.swift_version    = '5.0'
end
