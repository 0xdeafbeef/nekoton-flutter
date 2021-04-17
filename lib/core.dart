import 'dart:async';
import 'dart:ffi';
import 'dart:io';
import 'dart:isolate';
import 'dart:typed_data';

import 'package:async/async.dart';
import 'package:ffi/ffi.dart';
import 'package:stream_channel/isolate_channel.dart';
import 'package:stream_channel/stream_channel.dart';

import './bindings.dart' as nt;

const DYNAMIC_LIBRARY_FILE_NAME = "libntbindings.so";

class Core {
  static final nt.NekotonBindings _bindings =
      nt.NekotonBindings(Core._loadLibrary());

  static DynamicLibrary _loadLibrary() {
    return Platform.isAndroid
        ? DynamicLibrary.open(DYNAMIC_LIBRARY_FILE_NAME)
        : DynamicLibrary.process();
  }

  /// Computes a greeting for the given name using the native function
  static String greet(String name) {
    final ptrName = name.toNativeUtf8().cast<Int8>();

    // Native call
    final ptrResult = _bindings.rust_greeting(ptrName);

    // Cast the result pointer to a Dart string
    final result = ptrResult.cast<Utf8>().toDartString();

    // Clone the given result, so that the original string can be freed
    final resultCopy = "" + result;

    // Free the native value
    Core._free(result);

    return resultCopy;
  }

  /// Releases the memory allocated to handle the given (result) value
  static void _free(String value) {
    final ptr = value.toNativeUtf8().cast<Int8>();
    return _bindings.rust_cstr_free(ptr);
  }
}

class CoreIsolate {
  final SendPort connectPort;

  CoreIsolate.fromConnectPort(this.connectPort);

  StreamChannel _open() {
    final receive = ReceivePort('nekoton client receive');
    connectPort.send(receive.sendPort);

    final controller =
        StreamChannelController(allowForeignErrors: false, sync: true);
    receive.listen((message) {
      if (message is SendPort) {
        controller.local.stream
            .map(_prepareForTransport)
            .listen(message.send, onDone: receive.close);
      } else {
        controller.local.sink.add(_decodeAfterTransport(message));
      }
    });

    return controller.foreign;
  }

  static Future<CoreIsolate> spawn() async {
    final receiveServer = ReceivePort();
    final keyFuture = receiveServer.first;

    await Isolate.spawn(_startNekotonIsolate, [receiveServer.sendPort]);
    final key = await keyFuture as SendPort;
    return CoreIsolate.fromConnectPort(key);
  }

  factory CoreIsolate.inCurrent() {
    final server = _RunningNekotonServer();
    return CoreIsolate.fromConnectPort(server.portToOpenConnection);
  }
}

void _startNekotonIsolate(List args) {
  final sendPort = args[0] as SendPort;

  final server = _RunningNekotonServer();
  sendPort.send(server.portToOpenConnection);
}

class NekotonConnection {}

class _RunningNekotonServer {
  final NekotonServer server;
  final ReceivePort connectPort = ReceivePort('nekoton connect');
  int _counter = 0;

  SendPort get portToOpenConnection => connectPort.sendPort;

  _RunningNekotonServer() : server = NekotonServer() {
    final subscription = connectPort.listen((message) {
      if (message is SendPort) {
        final receiveForConnection =
            ReceivePort('nekoton channel #${_counter++}');

        message.send(receiveForConnection.sendPort);
        final channel = IsolateChannel(receiveForConnection, message)
            .changeStream((source) => source.map(_decodeAfterTransport))
            .transformSink(
              StreamSinkTransformer.fromHandlers(
                  handleData: (data, sink) =>
                      sink.add(_prepareForTransport(data))),
            );

        server.serve(channel);
      }
    });

    server.done.then((_) {
      subscription.cancel();
      connectPort.close();
    });
  }
}

Object? _prepareForTransport(Object? source) {
  if (source is! List) return source;

  if (source is Uint8List) {
    return TransferableTypedData.fromList([source]);
  }

  return source.map(_prepareForTransport).toList();
}

Object? _decodeAfterTransport(Object? source) {
  if (source is TransferableTypedData) {
    return source.materialize().asUint8List();
  } else if (source is List) {
    return source.map(_decodeAfterTransport).toList();
  } else {
    return source;
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

    // TODO: add connection
  }

  @override
  Future<void> shutdown() {
    _isShuttingDown = true;
    return done;
  }
}

class Runtime {
  late Pointer<nt.Runtime> _runtime;

  Runtime(int workerThreads) {
    Pointer<nt.RuntimeParams> params = calloc();
    Pointer<Pointer<nt.Runtime>> runtimeOut = calloc();

    params.ref.worker_threads = workerThreads;

    final int resultCode =
        Core._bindings.create_runtime(params.ref, runtimeOut);
    if (resultCode != 0) {
      calloc.free(params);
      throw Exception("Failed to create runtime");
    }

    _runtime = runtimeOut.value;

    calloc.free(params);
    calloc.free(runtimeOut);
  }

  void stop() {
    if (_runtime.address == nullptr.address) {
      return;
    }
    Core._bindings.delete_runtime(_runtime);
  }
}
