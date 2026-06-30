import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../bridge/bridge.dart';
import 'bridge_provider.dart';

final calendarsProvider = AsyncNotifierProvider<CalendarsNotifier, List<CalendarView>>(
  CalendarsNotifier.new,
);

class CalendarsNotifier extends AsyncNotifier<List<CalendarView>> {
  @override
  Future<List<CalendarView>> build() => ref.read(bridgeProvider).listCalendars();

  Future<void> createLocal(String displayName) async {
    await ref.read(bridgeProvider).createLocalCalendar(displayName);
    ref.invalidateSelf();
  }

  Future<List<String>> addCalDav({
    required String baseUrl,
    required String username,
    required String password,
  }) async {
    final ids = await ref.read(bridgeProvider).addCalDavAccount(
      baseUrl: baseUrl,
      username: username,
      password: password,
    );
    ref.invalidateSelf();
    return ids;
  }

  Future<SyncResult> syncCalendar(String calendarId) =>
      ref.read(bridgeProvider).syncNow(calendarId);
}

/// Which calendar IDs are currently visible on the calendar screen.
/// Starts with all calendars visible.
final visibleCalendarIdsProvider = StateProvider<Set<String>>((ref) {
  final cals = ref.watch(calendarsProvider).valueOrNull ?? [];
  return cals.map((c) => c.id).toSet();
});
