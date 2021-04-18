import 'dart:async';
import 'dart:developer';
import 'dart:ffi';
import 'dart:io';
import 'dart:isolate';

import 'package:ffi/ffi.dart';
import 'package:stream_channel/isolate_channel.dart';
import 'package:stream_channel/stream_channel.dart';

import './bindings.dart' as nt;

export './bindings.dart' show ContractType;

const DYNAMIC_LIBRARY_FILE_NAME = "libntbindings.so";

class _Nekoton {
  static final nt.NekotonBindings bindings = _Nekoton._loadLibrary();

  static nt.NekotonBindings _loadLibrary() {
    final library = Platform.isAndroid
        ? DynamicLibrary.open(DYNAMIC_LIBRARY_FILE_NAME)
        : DynamicLibrary.process();

    final bindings = nt.NekotonBindings(library);
    bindings.init(NativeApi.postCObject.cast());
    return bindings;
  }
}

class NekotonIsolate {
  final SendPort connectPort;

  NekotonIsolate.fromConnectPort(this.connectPort);

  Future<void> wait(int seconds) async {
    final ReceivePort callbackPort = ReceivePort();
    connectPort.send(callbackPort.sendPort);

    Completer<void> done = new Completer<void>();
    callbackPort.listen((message) {
      if (message is SendPort) {
        log("Got connection message");
        message.send(CmdWait(seconds, callbackPort.sendPort));
      } else {
        log("Finished waiting");
        done.complete();
      }
    });

    return done.future;
  }

  Future<TonWalletSubscription> subscribe(
      String publicKey, int contractType) async {
    final ReceivePort callbackPort = ReceivePort();
    connectPort.send(callbackPort.sendPort);

    final ReceivePort notificationPort = ReceivePort();

    Completer<TonWalletSubscription> done =
        new Completer<TonWalletSubscription>();
    callbackPort.listen((message) {
      if (message is SendPort) {
        log("Got connection message");
        message.send(
            CmdSubscribe(publicKey, contractType, notificationPort.sendPort));
      } else if (message is int) {
        log("Finished subscription");
        done.complete(TonWalletSubscription(notificationPort, message));
      }
    });

    return done.future;
  }

  static Future<NekotonIsolate> spawn() async {
    final receiveServer = ReceivePort();
    await Isolate.spawn(_startNekotonIsolate, [receiveServer.sendPort]);
    final connectPort = await receiveServer.first as SendPort;
    return NekotonIsolate.fromConnectPort(connectPort);
  }
}

void _startNekotonIsolate(List args) {
  final sendPort = args[0] as SendPort;

  final server = _RunningNekotonServer();
  sendPort.send(server.portToOpenConnection);
}

class _RunningNekotonServer {
  final NekotonServer server;
  final ReceivePort connectPort = ReceivePort();
  int _counter = 0;

  SendPort get portToOpenConnection => connectPort.sendPort;

  _RunningNekotonServer() : server = NekotonServer() {
    final subscription = connectPort.listen((message) {
      if (message is SendPort) {
        final receiveForConnection =
            ReceivePort('nekoton channel #${_counter++}');

        message.send(receiveForConnection.sendPort);
        final channel = IsolateChannel(receiveForConnection, message);

        log("Starting serve");
        server.serve(channel);
      }
    });

    server.done.then((_) {
      subscription.cancel();
      connectPort.close();
    });
  }
}

abstract class NekotonServer {
  factory NekotonServer() {
    return _NekotonServerImplementation();
  }

  Future<void> get done;

  void serve(StreamChannel<Object?> channel);

  Future<void> shutdown();
}

class _NekotonServerImplementation implements NekotonServer {
  final Runtime runtime;
  final Transport transport;

  bool _isShuttingDown = false;
  final Completer<void> _done = Completer();

  _NekotonServerImplementation()
      : runtime = Runtime(1),
        transport = Transport("https://main.ton.dev/graphql");

  @override
  Future<void> get done => _done.future;

  @override
  void serve(StreamChannel<Object?> channel) {
    if (_isShuttingDown) {
      throw StateError('Cannot add new channels after shutdown() was called');
    }

    final subscription = channel.stream.listen((cmd) async {
      if (cmd is CmdWait) {
        await runtime.wait(cmd.seconds);
        channel.sink.add(true);
      } else if (cmd is CmdSubscribe) {
        channel.sink.add(await subscribe(cmd));
      }
    });

    done.then((value) => subscription.cancel());

    // TODO: add connection
  }

  Future<int> subscribe(CmdSubscribe cmd) async {
    final resultPort = ReceivePort();

    final resultCode = _Nekoton.bindings.subscribe_to_ton_wallet(
        runtime._handle,
        transport._handle,
        cmd.publicKey.toNativeUtf8().cast(),
        cmd.contractType,
        cmd.notificationPort.nativePort,
        resultPort.sendPort.nativePort);
    if (resultCode != nt.ExitCode.Ok) {
      throw Exception("Failed to initiate subscription to ton wallet");
    }

    final subscriptionResult = await resultPort.first as List;
    if (subscriptionResult[0] as int != nt.ExitCode.Ok) {
      throw Exception("Failed to subscribe to ton wallet");
    }

    return subscriptionResult[1];
  }

  @override
  Future<void> shutdown() {
    _isShuttingDown = true;
    return done;
  }
}

class CmdWait {
  final int seconds;
  final SendPort sendPort;

  CmdWait(this.seconds, this.sendPort);
}

class CmdSubscribe {
  final String publicKey;
  final int contractType;
  final SendPort notificationPort;

  CmdSubscribe(this.publicKey, this.contractType, this.notificationPort);
}

class Runtime {
  late Pointer<nt.Runtime> _handle;

  Runtime(int workerThreads) {
    Pointer<nt.RuntimeParams> params = calloc();
    Pointer<Pointer<nt.Runtime>> runtimeOut = calloc();

    params.ref.worker_threads = workerThreads;

    final success = _Nekoton.bindings.create_runtime(params.ref, runtimeOut) ==
        nt.ExitCode.Ok;
    if (success) {
      _handle = runtimeOut.value;
    }

    calloc.free(params);
    calloc.free(runtimeOut);

    if (!success) {
      throw Exception("Failed to create runtime");
    }
  }

  Future<void> wait(int seconds) {
    final receivePort = ReceivePort();

    final resultCode = _Nekoton.bindings
        .wait(_handle, seconds, receivePort.sendPort.nativePort);
    if (resultCode != nt.ExitCode.Ok) {
      log("Not ok");
      throw Exception("Failed to wait");
    }

    log("Waiting...");
    return receivePort.first;
  }

  void stop() {
    if (_Nekoton.bindings.delete_runtime(_handle) != nt.ExitCode.Ok) {
      throw Exception("Failed to delete runtime");
    }
  }
}

class Transport {
  late Pointer<nt.GqlTransport> _handle;

  Transport(String url) {
    Pointer<nt.TransportParams> params = calloc();
    Pointer<Pointer<nt.GqlTransport>> transportOut = calloc();

    params.ref.url = url.toNativeUtf8().cast();

    final success =
        _Nekoton.bindings.create_gql_transport(params.ref, transportOut) ==
            nt.ExitCode.Ok;
    if (success) {
      _handle = transportOut.value;
    }

    calloc.free(params.ref.url);
    calloc.free(params);
    calloc.free(transportOut);

    if (!success) {
      throw Exception("Failed to create transport");
    }
  }

  void delete() {
    if (_Nekoton.bindings.delete_gql_transport(_handle) != nt.ExitCode.Ok) {
      throw Exception("Failed to delete transport");
    }
  }
}

class TonWalletSubscription {
  late int _handle;
  late ReceivePort _notificationPort;

  TonWalletSubscription(this._notificationPort, this._handle);

  Stream<int> get balance {
    return _notificationPort.cast();
  }

  void delete() {
    final handle = Pointer<nt.TonWalletSubscription>.fromAddress(_handle);
    final resultCode = _Nekoton.bindings.delete_subscription(handle);
    if (resultCode != nt.ExitCode.Ok) {
      throw Exception("Failed to delete ton wallet subscription");
    }
  }
}
