import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:path/path.dart' as p;
import 'package:path_provider/path_provider.dart';

import 'platform/notifications.dart';
import 'presentation/calendar_list_screen.dart';
import 'presentation/calendar_screen.dart';
import 'state/bridge_provider.dart';
import 'theme/app_theme.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await initNotifications();

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
      theme: AppTheme.light(),
      darkTheme: AppTheme.dark(),
      routerConfig: _router,
    );
  }
}

final _initProvider = FutureProvider<void>((ref) async {
  final bridge = ref.read(bridgeProvider);
  final docsDir = await getApplicationDocumentsDirectory();
  await bridge.init(p.join(docsDir.path, 'songbird.sqlite3'));
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
    NavigationDestination(
      icon: Icon(Icons.calendar_today_outlined),
      selectedIcon: Icon(Icons.calendar_today),
      label: 'Calendar',
    ),
    NavigationDestination(
      icon: Icon(Icons.layers_outlined),
      selectedIcon: Icon(Icons.layers),
      label: 'Calendars',
    ),
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
