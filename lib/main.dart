import 'dart:developer';

import 'package:flutter/material.dart';
import 'package:nekoton/core.dart';

void main() {
  runApp(MyApp());
}

class MyApp extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Nekoton',
      theme: ThemeData(
        primarySwatch: Colors.orange,
      ),
      home: MyHomePage(title: 'Flutter Demo Home Page'),
    );
  }
}

class MyHomePage extends StatefulWidget {
  MyHomePage({Key? key, required this.title}) : super(key: key);

  final String title;

  @override
  _MyHomePageState createState() => _MyHomePageState();
}

class _MyHomePageState extends State<MyHomePage> {
  String _text = '';
  TonWalletSubscription? _subscription;
  late NekotonIsolate core;

  void _subscribe() async {
    final subscription = await core.subscribe(
        "1161f67ca580dd2b9935967b04109e0e988601fc0894e145f7cd56534e817257",
        ContractType.WalletV3);
    setState(() => {_subscription = subscription});

    subscription.balance.listen((balance) {
      setState(() {
        _text = balance.toString();
      });
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: <Widget>[
            Text(
              'Your balance',
            ),
            Text(
              '$_text',
              style: Theme.of(context).textTheme.headline4,
            ),
          ],
        ),
      ),
      floatingActionButton: (_subscription == null
          ? FloatingActionButton(
              onPressed: _subscribe,
              tooltip: 'Subscribe',
              child: Icon(Icons.add),
            )
          : null),
    );
  }

  @override
  void initState() {
    log("starting runtime...");
    NekotonIsolate.spawn().then((value) {
      core = value;
      log("started runtime");
    });

    super.initState();
  }

  @override
  void dispose() {
    super.dispose();
  }
}
