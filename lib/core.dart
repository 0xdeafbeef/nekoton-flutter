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
  late WalletContext ctx;

  NekotonIsolate(String url, String pubkey, nt.ContractType contract_type,
      String keystoreData) {
    ctx = WalletContext(url, pubkey, contract_type, keystoreData);
  }

  Future<void> send_tons(
      int amount, String password, String to, String? comment) async {
    ReceivePort isolateToMainStream = ReceivePort();

    Pointer<Int8> ffi_comment;
    if (comment != null) {
      ffi_comment = comment.toNativeUtf8().cast();
    } else {
      ffi_comment = Pointer.fromAddress(0); //todo use some native code
    }
    _Nekoton.bindings.send(ctx._handle, sign_data, isolateToMainStream., ffi_comment,
        to.toNativeUtf8().cast(), amount);
    {}
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
  Runtime();

  Future<void> wait(int seconds) {
    final receivePort = ReceivePort();

    final resultCode =
        _Nekoton.bindings.wait(seconds, receivePort.sendPort.nativePort);
    if (resultCode != nt.ExitCode.Ok) {
      log("Not ok");
      throw Exception("Failed to wait");
    }

    log("Waiting...");
    return receivePort.first;
  }
}

enum ContractType {
  SafeMultisig,
  SafeMultisig24h,
  SetcodeMultisig,
  Surf,
  WalletV3,
}

class WalletContext {
  late Pointer<nt.Context> _handle;
  late ReceivePort _notificationPort;

  WalletContext(String url, String pubkey, nt.ContractType contract_type,
      String keystoreData) {
    Pointer<nt.TransportParams> params = calloc();
    params.ref.url = url.toNativeUtf8().cast();
    int contractType = contract_type as int;
    Pointer<Pointer<nt.Context>> ContexOut = calloc();
    int res = _Nekoton.bindings.create_context(
        params.ref,
        pubkey.toNativeUtf8().cast(),
        contractType,
        this._notificationPort.sendPort.nativePort,
        keystoreData.toNativeUtf8().cast(),
        ContexOut);

    if (res == nt.ExitCode.Ok) {
      _handle = ContexOut.value;
    } else {
      throw Exception('failed to create context with code $res');
    }
  }

  Stream<String> get update {
    return _notificationPort.cast();
  }
}
