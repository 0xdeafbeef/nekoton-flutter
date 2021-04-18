import 'dart:async';
import 'dart:developer';
import 'dart:ffi';
import 'dart:io';
import 'dart:isolate';

import 'package:ffi/ffi.dart';
import 'package:stream_channel/isolate_channel.dart';
import 'package:stream_channel/stream_channel.dart';

import './bindings.dart' as nt;

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

    await done.future;
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

  bool _isShuttingDown = false;
  final Completer<void> _done = Completer();

  _NekotonServerImplementation() : runtime = Runtime(1);

  @override
  Future<void> get done => _done.future;

  @override
  void serve(StreamChannel<Object?> channel) {
    if (_isShuttingDown) {
      throw StateError('Cannot add new channels after shutdown() was called');
    }

    log("Connected to channel");

    final subscription = channel.stream.listen((event) async {
      log("Got event");
      if (event is CmdWait) {
        log("Got cmd wait: ${event.seconds}");
        await runtime.wait(event.seconds);
        channel.sink.add(true);
      }
    });

    done.then((value) => subscription.cancel());

    // TODO: add connection
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

class Runtime {
  late Pointer<nt.Runtime> _runtime;

  Runtime(int workerThreads) {
    Pointer<nt.RuntimeParams> params = calloc();
    Pointer<Pointer<nt.Runtime>> runtimeOut = calloc();

    params.ref.worker_threads = workerThreads;

    final int resultCode =
        _Nekoton.bindings.create_runtime(params.ref, runtimeOut);
    if (resultCode != nt.ExitCode.Ok) {
      calloc.free(params);
      calloc.free(runtimeOut);
      throw Exception("Failed to create runtime");
    }

    _runtime = runtimeOut.value;

    calloc.free(params);
    calloc.free(runtimeOut);
  }

  Future<void> wait(int seconds) {
    final receivePort = ReceivePort();

    final resultCode = _Nekoton.bindings
        .wait(_runtime, seconds, receivePort.sendPort.nativePort);
    if (resultCode != nt.ExitCode.Ok) {
      log("Not ok");
      throw Exception("Failed to wait");
    }

    log("Waiting...");
    return receivePort.first;
  }

  void stop() {
    final int resultCode = _Nekoton.bindings.delete_runtime(_runtime);
    if (resultCode != nt.ExitCode.Ok) {
      throw Exception("Failed to delete runtime");
    }
  }
}
