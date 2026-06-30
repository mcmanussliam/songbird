import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../bridge/bridge.dart';
import '../bridge/bridge_stub.dart';

/// Provides the active Bridge implementation.
///
/// Override in main() or tests by passing an [Override]:
///   runApp(ProviderScope(overrides: [bridgeProvider.overrideWithValue(myBridge)], child: ...))
final bridgeProvider = Provider<Bridge>((ref) => BridgeStub());
