import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'platform/notifications.dart';
import 'state/bridge_provider.dart';
import 'presentation/calendar_screen.dart';
import 'presentation/calendar_list_screen.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await initNotifications();

  // TODO(M3 setup): replace BridgeStub with BridgeFrb once
  // flutter_rust_bridge_codegen generate has been run in app/.
  // Until then the app runs on stub data so UI can be developed without
  // the generated bindings.
  runApp(const ProviderScope(child: SongbirdApp()));
}

final _router = GoRouter(
  routes: [
    ShellRoute(
      builder: (context, state, child) => _AppShell(child: child),
      routes: [
        GoRoute(
          path: '/',
          pageBuilder: (context, state) => const NoTransitionPage(
            child: CalendarScreen(),
          ),
        ),
        GoRoute(
          path: '/calendars',
          pageBuilder: (context, state) => const NoTransitionPage(
            child: CalendarListScreen(),
          ),
        ),
      ],
    ),
  ],
);

class SongbirdApp extends ConsumerWidget {
  const SongbirdApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // Initialise the bridge on first build.
    ref.watch(_initProvider);

    return MaterialApp.router(
      title: 'Songbird',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: const Color(0xFF4A90D9)),
        useMaterial3: true,
      ),
      routerConfig: _router,
    );
  }
}

final _initProvider = FutureProvider<void>((ref) async {
  final bridge = ref.read(bridgeProvider);
  // For BridgeFrb the db path comes from path_provider; stub ignores it.
  await bridge.init(':memory:');
});

class _AppShell extends StatefulWidget {
  const _AppShell({required this.child});
  final Widget child;

  @override
  State<_AppShell> createState() => _AppShellState();
}

class _AppShellState extends State<_AppShell> {
  int _index = 0;

  static const _destinations = [
    NavigationDestination(icon: Icon(Icons.calendar_month), label: 'Calendar'),
    NavigationDestination(icon: Icon(Icons.list), label: 'Calendars'),
  ];

  static const _routes = ['/', '/calendars'];

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: widget.child,
      bottomNavigationBar: NavigationBar(
        selectedIndex: _index,
        onDestinationSelected: (i) {
          setState(() => _index = i);
          context.go(_routes[i]);
        },
        destinations: _destinations,
      ),
    );
  }
}
