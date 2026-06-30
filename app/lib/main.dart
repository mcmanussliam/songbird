// Songbird — Flutter entrypoint.
//
// See system-design.md §6.2 for the layering rule this file's structure depends on:
//   presentation/  -> widgets only, no bridge calls
//   state/         -> Riverpod providers, owns all bridge calls and stream subscriptions
//   platform/      -> platform channel code (push, widgets, share sheet, deep links)
//   plugin_api/    -> Dart-side plugin extension points (§13), stubbed until Phase 3
//
// TODO(M3): replace this placeholder with the real app shell once songbird-core (M1) and
// the flutter_rust_bridge codegen setup (rust_bridge/) are in place.

import 'package:flutter/material.dart';

void main() {
  runApp(const SongbirdApp());
}

class SongbirdApp extends StatelessWidget {
  const SongbirdApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Songbird',
      home: Scaffold(
        body: Center(
          child: Text(
            'Songbird — M1 in progress.\n'
            'See docs/design/system-design.md §14 for milestone status.',
            textAlign: TextAlign.center,
          ),
        ),
      ),
    );
  }
}
