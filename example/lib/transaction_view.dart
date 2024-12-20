import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_libmwc_example/init_transaction_view.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';

class TransactionView extends StatelessWidget {
  TransactionView({Key? key, required this.password}) : super(key: key);

  final String password;

  // This widget is the root of your application.
  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Wallet Transactions',
      theme: ThemeData(
        // This is the theme of your application.
        //
        // Try running your application with "flutter run". You'll see the
        // application has a blue toolbar. Then, without quitting the app, try
        // changing the primarySwatch below to Colors.green and then invoke
        // "hot reload" (press "r" in the console where you ran "flutter run",
        // or simply save your changes to "hot reload" in a Flutter IDE).
        // Notice that the counter didn't reset back to zero; the application
        // is not restarted.
        primarySwatch: Colors.blue,
      ),
      home: MwcTransactionView(
        title: 'Transactions',
        password: password,
      ),
    );
  }
}

class MwcTransactionView extends StatefulWidget {
  final String password;

  const MwcTransactionView(
      {Key? key, required this.title, required this.password})
      : super(key: key);

  // This widget is the home page of your application. It is stateful, meaning
  // that it has a State object (defined below) that contains fields that affect
  // how it looks.

  // This class is the configuration for the state. It holds the values (in this
  // case the title) provided by the parent (in this case the App widget) and
  // used by the build method of the State. Fields in a Widget subclass are
  // always marked "final".

  final String title;

  @override
  State<MwcTransactionView> createState() => _MwcTransactionView();
}

class _MwcTransactionView extends State<MwcTransactionView> {
  String walletConfig = "";
  final storage = const FlutterSecureStorage();

  Future<void> _getWalletConfig() async {
    var config = await storage.read(key: "config");
    String strConf = json.encode(config);

    setState(() {
      walletConfig = strConf;
    });
  }

  final _formKey = GlobalKey<FormState>();
  @override
  Widget build(BuildContext context) {
    _getWalletConfig();
    String password = widget.password;

    // print("Wallet Config");
    // print(json.decode(walletConfig));
    String decodeConfig = json.decode(walletConfig);
    const refreshFromNode = 0;

    String walletInfo = "fixme";
    //  getWalletInfo(decodeConfig, password, refreshFromNode);
    var data = json.decode(walletInfo);

    var total = data['total'].toString();
    var awaitingFinalisation = data['amount_awaiting_finalization'].toString();
    var awaitingConfirmation = data['amount_awaiting_confirmation'].toString();
    var spendable = data['amount_currently_spendable'].toString();
    var locked = data['amount_locked'].toString();

    const minimumConfirmations = 10;

    String transactions = "fixme";
    // getTransactions(
    //     decodeConfig, password, minimumConfirmations, refreshFromNode);

    print("List Transactions count");
    print(transactions);

    return Scaffold(
      appBar: AppBar(
        // Here we take the value from the MyHomePage object that was created by
        // the App.build method, and use it to set our appbar title.
        title: Text(widget.title),
      ),
      body: Center(
        child: Column(
          children: <Widget>[
            Text("Total Amount : $total"),
            Text("Amount Awaiting Finalization : $awaitingFinalisation"),
            Text("Amount Awaiting Confirmation : $awaitingConfirmation"),
            Text("Amount Currently Spendable : $spendable"),
            Text("Amount Locked : $locked"),
            ElevatedButton(
                onPressed: () {
                  Navigator.push(
                    context,
                    MaterialPageRoute(
                        builder: (context) => InitTransactionView(
                              password: password,
                            )),
                  );
                },
                child: const Text("Init Transaction")),
            // Expanded(
            //   child: ListView.builder(
            //     itemCount: transactions.length,
            //     itemBuilder: (context, index) {
            //       return ListTile(
            //         title: Text(transactions),
            //       );
            //     },
            //   ),
            // ),

            // Add TextFormFields and ElevatedButton here.
          ],
        ),
      ),
    );
  }
}
