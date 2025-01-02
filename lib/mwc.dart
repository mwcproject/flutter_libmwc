
import 'dart:ffi';
import 'dart:io' as io;

import 'package:ffi/ffi.dart';

final DynamicLibrary mwcNative = io.Platform.isWindows
    ? DynamicLibrary.open("libmwc_wallet.dll") // Windows
    : io.Platform.environment.containsKey('FLUTTER_TEST')
        ? DynamicLibrary.open(
            'crypto_plugins/flutter_libmwc/scripts/linux/build/libmwc_wallet.so') // Flutter tests
        : io.Platform.isAndroid || io.Platform.isLinux
            ? DynamicLibrary.open('libmwc_wallet.so') // Android/Linux
            : io.Platform.isMacOS
                ? DynamicLibrary.open(
                    'MwcWallet.framework/MwcWallet') // macOS
                : io.Platform.isIOS
                    ? DynamicLibrary.open(
                        'MwcWallet.framework/MwcWallet') // iOS
                    : DynamicLibrary.process();

typedef WalletMnemonic = Pointer<Utf8> Function();
typedef WalletMnemonicFFI = Pointer<Utf8> Function();


typedef InitLogs = Pointer<Utf8> Function(Pointer<Utf8>);
typedef InitLogsFFI = Pointer<Utf8> Function(Pointer<Utf8>);

typedef WalletInit = Pointer<Utf8> Function(
    Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>);
typedef WalletInitFFI = Pointer<Utf8> Function(
    Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>);

typedef WalletInfo = Pointer<Utf8> Function(
    Pointer<Utf8>, Pointer<Int8>, Pointer<Int8>);
typedef WalletInfoFFI = Pointer<Utf8> Function(
    Pointer<Utf8>, Pointer<Int8>, Pointer<Int8>);

typedef RecoverWallet = Pointer<Utf8> Function(
    Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>);
typedef RecoverWalletFFI = Pointer<Utf8> Function(
    Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>);

typedef WalletPhrase = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>);
typedef WalletPhraseFFI = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>);

typedef ScanOutPuts = Pointer<Utf8> Function(
    Pointer<Utf8>, Pointer<Int8>, Pointer<Int8>);
typedef ScanOutPutsFFI = Pointer<Utf8> Function(
    Pointer<Utf8>, Pointer<Int8>, Pointer<Int8>);

typedef CreateTransaction = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Int8>,
    Pointer<Utf8>, Pointer<Int8>, Pointer<Utf8>, Pointer<Int8>, Pointer<Utf8>);
typedef CreateTransactionFFI = Pointer<Utf8> Function(Pointer<Utf8>,
    Pointer<Int8>, Pointer<Utf8>, Pointer<Int8>, Pointer<Utf8>, Pointer<Int8>,
    Pointer<Utf8>);

typedef MwcMqsListenerStart = Pointer<Void> Function(
    Pointer<Utf8>, Pointer<Utf8>);
typedef MwcMqsListenerStartFFI = Pointer<Void> Function(
    Pointer<Utf8>, Pointer<Utf8>);

typedef MwcMqsListenerStop = Pointer<Utf8> Function(Pointer<Void>);
typedef MwcMqsListenerStopFFI = Pointer<Utf8> Function(Pointer<Void>);

typedef GetTransactions = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Int8>);
typedef GetTransactionsFFI = Pointer<Utf8> Function(
    Pointer<Utf8>, Pointer<Int8>);

typedef CancelTransaction = Pointer<Utf8> Function(
    Pointer<Utf8>, Pointer<Utf8>);
typedef CancelTransactionFFI = Pointer<Utf8> Function(
    Pointer<Utf8>, Pointer<Utf8>);

typedef GetChainHeight = Pointer<Utf8> Function(Pointer<Utf8>);
typedef GetChainHeightFFI = Pointer<Utf8> Function(Pointer<Utf8>);

typedef AddressInfo = Pointer<Utf8> Function(
    Pointer<Utf8>, Pointer<Int8>);
typedef AddressInfoFFI = Pointer<Utf8> Function(
    Pointer<Utf8>, Pointer<Int8>);

typedef ValidateAddress = Pointer<Utf8> Function(Pointer<Utf8>);
typedef ValidateAddressFFI = Pointer<Utf8> Function(Pointer<Utf8>);



typedef TransactionFees = Pointer<Utf8> Function(
    Pointer<Utf8>, Pointer<Int8>, Pointer<Int8>);
typedef TransactionFeesFFI = Pointer<Utf8> Function(
    Pointer<Utf8>, Pointer<Int8>, Pointer<Int8>);

typedef DeleteWallet = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>);
typedef DeleteWalletFFI = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>);

typedef OpenWallet = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>);
typedef OpenWalletFFI = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>);

typedef TxHttpSend = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Int8>,
    Pointer<Int8>, Pointer<Utf8>, Pointer<Int8>, Pointer<Utf8>);
typedef TxHttpSendFFI = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Int8>,
    Pointer<Int8>, Pointer<Utf8>, Pointer<Int8>, Pointer<Utf8>);

typedef TxSlatepackSend = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Int8>,
    Pointer<Int8>, Pointer<Utf8>, Pointer<Int8>, Pointer<Utf8>);
typedef TxSlatepackSendFFI = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Int8>,
    Pointer<Int8>, Pointer<Utf8>, Pointer<Int8>, Pointer<Utf8>);

final WalletMnemonic _walletMnemonic = mwcNative
    .lookup<NativeFunction<WalletMnemonicFFI>>("get_mnemonic")
    .asFunction();

String walletMnemonic() {
  return _walletMnemonic().toDartString();
}


final InitLogs _initLogs = mwcNative
    .lookup<NativeFunction<InitLogsFFI>>("rust_init_logs")
    .asFunction();

String initLogs( String config) {
  return _initLogs(config.toNativeUtf8())
      .toDartString();
}

final WalletInit _initWallet = mwcNative
    .lookup<NativeFunction<WalletInitFFI>>("wallet_init")
    .asFunction();

String initWallet(
    String config, String mnemonic, String password, String name) {
  return _initWallet(config.toNativeUtf8(), mnemonic.toNativeUtf8(),
      password.toNativeUtf8(), name.toNativeUtf8())
      .toDartString();
}

final WalletInfo _walletInfo = mwcNative
    .lookup<NativeFunction<WalletInfoFFI>>("rust_wallet_balances")
    .asFunction();

Future<String> getWalletInfo(
    String wallet, int refreshFromNode, int min_confirmations) async {
  return _walletInfo(
      wallet.toNativeUtf8(),
      refreshFromNode.toString().toNativeUtf8().cast<Int8>(),
      min_confirmations.toString().toNativeUtf8().cast<Int8>())
      .toDartString();
}

final RecoverWallet _recoverWallet = mwcNative
    .lookup<NativeFunction<RecoverWalletFFI>>("rust_recover_from_mnemonic")
    .asFunction();

String recoverWallet(
    String config, String password, String mnemonic, String name) {
  return _recoverWallet(config.toNativeUtf8(), password.toNativeUtf8(),
      mnemonic.toNativeUtf8(), name.toNativeUtf8())
      .toDartString();
}

final ScanOutPuts _scanOutPuts = mwcNative
    .lookup<NativeFunction<ScanOutPutsFFI>>("rust_wallet_scan_outputs")
    .asFunction();

Future<String> scanOutPuts(
    String wallet, int startHeight, int numberOfBlocks) async {
  return _scanOutPuts(
    wallet.toNativeUtf8(),
    startHeight.toString().toNativeUtf8().cast<Int8>(),
    numberOfBlocks.toString().toNativeUtf8().cast<Int8>(),
  ).toDartString();
}

final MwcMqsListenerStart _MwcMqsListenerStart = mwcNative
    .lookup<NativeFunction<MwcMqsListenerStartFFI>>(
    "rust_mwcmqs_listener_start")
    .asFunction();

Pointer<Void> mwcMqsListenerStart(String wallet, String MWCMQSConfig) {
  return _MwcMqsListenerStart(
    wallet.toNativeUtf8(),
    MWCMQSConfig.toNativeUtf8(),
  );
}

final MwcMqsListenerStop _MwcMqsListenerStop = mwcNative
    .lookup<NativeFunction<MwcMqsListenerStopFFI>>("_listener_cancel")
    .asFunction();

String mwcMqsListenerStop(Pointer<Void> handler) {
  return _MwcMqsListenerStop(
    handler,
  ).toDartString();
}

final CreateTransaction _createTransaction = mwcNative
    .lookup<NativeFunction<CreateTransactionFFI>>("rust_create_tx")
    .asFunction();

Future<String> createTransaction(String wallet, int amount, String address,
    int secretKey, String MWCMQSConfig, int minimumConfirmations, String note) async {
  return _createTransaction(
    wallet.toNativeUtf8(),
    amount.toString().toNativeUtf8().cast<Int8>(),
    address.toNativeUtf8(),
    secretKey.toString().toNativeUtf8().cast<Int8>(),
    MWCMQSConfig.toNativeUtf8(),
    minimumConfirmations.toString().toNativeUtf8().cast<Int8>(),
    note.toNativeUtf8(),
  ).toDartString();
}

final GetTransactions _getTransactions = mwcNative
    .lookup<NativeFunction<GetTransactionsFFI>>("rust_txs_get")
    .asFunction();

Future<String> getTransactions(String wallet, int refreshFromNode) async {
  return _getTransactions(wallet.toNativeUtf8(),
      refreshFromNode.toString().toNativeUtf8().cast<Int8>())
      .toDartString();
}

final CancelTransaction _cancelTransaction = mwcNative
    .lookup<NativeFunction<CancelTransactionFFI>>("rust_tx_cancel")
    .asFunction();

String cancelTransaction(String wallet, String transactionId) {
  return _cancelTransaction(wallet.toNativeUtf8(), transactionId.toNativeUtf8())
      .toDartString();
}

final GetChainHeight _getChainHeight = mwcNative
    .lookup<NativeFunction<GetChainHeightFFI>>("rust_get_chain_height")
    .asFunction();

int getChainHeight(String config) {
  String latestHeight = _getChainHeight(config.toNativeUtf8()).toDartString();
  return int.parse(latestHeight);
}

final AddressInfo _addressInfo = mwcNative
    .lookup<NativeFunction<AddressInfoFFI>>("rust_get_wallet_address")
    .asFunction();

String getAddressInfo(String wallet, int index) {
  return _addressInfo(
      wallet.toNativeUtf8(),
      index.toString().toNativeUtf8().cast<Int8>())
      .toDartString();
}

final ValidateAddress _validateSendAddress = mwcNative
    .lookup<NativeFunction<ValidateAddressFFI>>("rust_validate_address")
    .asFunction();

String validateSendAddress(String address) {
  return _validateSendAddress(address.toNativeUtf8()).toDartString();
}

typedef PreviewSlatepack = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>);
typedef PreviewSlatepackFFI = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>);

final PreviewSlatepack _previewSlatepackMessage = mwcNative
    .lookup<NativeFunction<PreviewSlatepackFFI>>("rust_preview_slatepack")
    .asFunction();

/// Function to preview a Slatepack message and parse the JSON result
String previewSlatepackMessage(String wallet, String message) {
  try {
    // Call the FFI function
    final resultPointer = _previewSlatepackMessage(wallet.toNativeUtf8(), message.toNativeUtf8());
    final resultString = resultPointer.toDartString();

    // Return the raw JSON result string for further processing
    return resultString;
  } catch (e) {
    // Handle any potential errors gracefully
    throw Exception("Error decoding Slatepack preview: ${e.toString()}");
  }
}

final TransactionFees _transactionFees = mwcNative
    .lookup<NativeFunction<TransactionFeesFFI>>("rust_get_tx_fees")
    .asFunction();

Future<String> getTransactionFees(
    String wallet, int amount, int minimumConfirmations) async {
  return _transactionFees(
      wallet.toNativeUtf8(),
      amount.toString().toNativeUtf8().cast<Int8>(),
      minimumConfirmations.toString().toNativeUtf8().cast<Int8>())
      .toDartString();
}

final DeleteWallet _deleteWallet = mwcNative
    .lookup<NativeFunction<DeleteWalletFFI>>("rust_delete_wallet")
    .asFunction();

Future<String> deleteWallet(String wallet, String config) async {
  return _deleteWallet(wallet.toNativeUtf8(), config.toNativeUtf8())
      .toDartString();
}

final OpenWallet _openWallet = mwcNative
    .lookup<NativeFunction<OpenWalletFFI>>("rust_open_wallet")
    .asFunction();

String openWallet(String config, String password) {
  return _openWallet(config.toNativeUtf8(), password.toNativeUtf8())
      .toDartString();
}

final TxHttpSend _txHttpSend = mwcNative
    .lookup<NativeFunction<TxHttpSendFFI>>("rust_tx_send_http")
    .asFunction();

Future<String> txHttpSend(
    String wallet,
    int selectionStrategyIsAll,
    int minimumConfirmations,
    String message,
    int amount,
    String address) async {
  return _txHttpSend(
      wallet.toNativeUtf8(),
      selectionStrategyIsAll.toString().toNativeUtf8().cast<Int8>(),
      minimumConfirmations.toString().toNativeUtf8().cast<Int8>(),
      message.toNativeUtf8(),
      amount.toString().toNativeUtf8().cast<Int8>(),
      address.toNativeUtf8())
      .toDartString();
}

final TxSlatepackSend _txSlatepackSend = mwcNative
    .lookup<NativeFunction<TxSlatepackSendFFI>>("rust_tx_send_slatepack")
    .asFunction();

Future<String> txSlatepackSend(
    String wallet,
    int selectionStrategyIsAll,
    int minimumConfirmations,
    String message,
    int amount,
    String address) async {
  return _txSlatepackSend(
      wallet.toNativeUtf8(),
      selectionStrategyIsAll.toString().toNativeUtf8().cast<Int8>(),
      minimumConfirmations.toString().toNativeUtf8().cast<Int8>(),
      message.toNativeUtf8(),
      amount.toString().toNativeUtf8().cast<Int8>(),
      address.toNativeUtf8())
      .toDartString();
}



