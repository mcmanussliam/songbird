import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../bridge/bridge.dart';
import 'bridge_provider.dart';
import 'calendar_providers.dart';

final selectedDateProvider = StateProvider<DateTime>((ref) {
  final now = DateTime.now();
  return DateTime(now.year, now.month, now.day);
});

final occurrencesProvider = FutureProvider.family<List<OccurrenceView>, (DateTime, DateTime)>(
  (ref, range) async {
    final bridge = ref.watch(bridgeProvider);
    final visibleIds = ref.watch(visibleCalendarIdsProvider);
    if (visibleIds.isEmpty) return [];
    return bridge.occurrencesInRange(
      calendarIds: visibleIds.toList(),
      start: range.$1,
      end: range.$2,
    );
  },
);

/// Occurrences for the currently displayed month (padded ±7 days for grid overflow).
final monthOccurrencesProvider = FutureProvider<List<OccurrenceView>>((ref) async {
  final selected = ref.watch(selectedDateProvider);
  final start = DateTime(selected.year, selected.month, 1).subtract(const Duration(days: 7));
  final end = DateTime(selected.year, selected.month + 1, 1).add(const Duration(days: 7));
  return ref.watch(occurrencesProvider((start, end)).future);
});

/// Occurrences for a specific day, used in day/agenda view.
final dayOccurrencesProvider = FutureProvider.family<List<OccurrenceView>, DateTime>(
  (ref, day) async {
    final start = DateTime(day.year, day.month, day.day);
    final end = start.add(const Duration(days: 1));
    final all = await ref.watch(occurrencesProvider((start, end)).future);
    return all..sort((a, b) => a.dtstart.compareTo(b.dtstart));
  },
);

class EventsNotifier extends AsyncNotifier<void> {
  @override
  Future<void> build() async {}

  Future<String> createEvent(String calendarId, EventDraft draft) async {
    final id = await ref.read(bridgeProvider).createEvent(calendarId, draft);
    ref.invalidate(occurrencesProvider);
    ref.invalidate(monthOccurrencesProvider);
    return id;
  }

  Future<void> updateEvent(String eventId, EventPatch patch) async {
    await ref.read(bridgeProvider).updateEvent(eventId, patch);
    ref.invalidate(occurrencesProvider);
    ref.invalidate(monthOccurrencesProvider);
  }

  Future<void> deleteEvent(String eventId, DeleteScope scope) async {
    await ref.read(bridgeProvider).deleteEvent(eventId, scope);
    ref.invalidate(occurrencesProvider);
    ref.invalidate(monthOccurrencesProvider);
  }
}

final eventsNotifierProvider = AsyncNotifierProvider<EventsNotifier, void>(EventsNotifier.new);
