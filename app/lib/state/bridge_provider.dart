import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../bridge/bridge.dart';
import '../bridge/bridge_frb.dart';

/// Provides the active Bridge implementation.
///
/// Override in main() or tests by passing an [Override]:
///   runApp(ProviderScope(overrides: [bridgeProvider.overrideWithValue(myBridge)], child: ...))
///
/// [BridgeStub] remains available in bridge_stub.dart for widget tests that want to avoid
/// loading the native library.
final bridgeProvider = Provider<Bridge>((ref) => BridgeFrb());
