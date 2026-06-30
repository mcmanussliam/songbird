/// Stub bridge for development and widget tests.
///
/// Returns mock data so the Flutter UI can be built and tested without the
/// flutter_rust_bridge generated bindings. Replace with BridgeFrb once
/// `flutter_rust_bridge_codegen generate` has been run.
library;

import 'bridge.dart';

class BridgeStub implements Bridge {
  final List<CalendarView> _calendars = [
    const CalendarView(id: 'cal-1', displayName: 'Personal', source: 'local'),
    const CalendarView(id: 'cal-2', displayName: 'Work', source: 'local'),
  ];

  final List<OccurrenceView> _events = [];
  bool _initialized = false;

  @override
  Future<void> init(String dbPath) async {
    _initialized = true;
    final now = DateTime.now();
    _events.addAll([
      OccurrenceView(
        eventId: 'ev-1',
        calendarId: 'cal-1',
        summary: 'Team standup',
        dtstart: DateTime(now.year, now.month, now.day, 9, 0),
        dtend: DateTime(now.year, now.month, now.day, 9, 30),
        isAllDay: false,
        hasRecurrence: true,
        status: 'confirmed',
      ),
      OccurrenceView(
        eventId: 'ev-2',
        calendarId: 'cal-1',
        summary: 'Lunch with Sam',
        dtstart: DateTime(now.year, now.month, now.day, 12, 30),
        dtend: DateTime(now.year, now.month, now.day, 13, 30),
        isAllDay: false,
        hasRecurrence: false,
        status: 'confirmed',
      ),
    ]);
  }

  @override
  Future<List<CalendarView>> listCalendars() async {
    _assertInit();
    return List.unmodifiable(_calendars);
  }

  @override
  Future<String> createLocalCalendar(String displayName) async {
    _assertInit();
    final id = 'cal-${_calendars.length + 1}';
    _calendars.add(CalendarView(id: id, displayName: displayName, source: 'local'));
    return id;
  }

  @override
  Future<List<String>> addCalDavAccount({
    required String baseUrl,
    required String username,
    required String password,
  }) async {
    _assertInit();
    final id = 'cal-caldav-${_calendars.length + 1}';
    _calendars.add(CalendarView(
      id: id,
      displayName: 'CalDAV: $baseUrl',
      source: 'caldav',
    ));
    return [id];
  }

  @override
  Future<List<OccurrenceView>> occurrencesInRange({
    required List<String> calendarIds,
    required DateTime start,
    required DateTime end,
  }) async {
    _assertInit();
    return _events.where((e) {
      return calendarIds.contains(e.calendarId) &&
          e.dtstart.isBefore(end) &&
          e.dtend.isAfter(start);
    }).toList();
  }

  @override
  Future<String> createEvent(String calendarId, EventDraft draft) async {
    _assertInit();
    final id = 'ev-${_events.length + 1}';
    _events.add(OccurrenceView(
      eventId: id,
      calendarId: calendarId,
      summary: draft.summary,
      description: draft.description,
      location: draft.location,
      dtstart: draft.dtstart,
      dtend: draft.dtend,
      isAllDay: draft.isAllDay,
      timezone: draft.timezone,
      hasRecurrence: draft.rrule != null,
      status: 'confirmed',
    ));
    return id;
  }

  @override
  Future<void> updateEvent(String eventId, EventPatch patch) async {
    _assertInit();
    final idx = _events.indexWhere((e) => e.eventId == eventId);
    if (idx == -1) return;
    final existing = _events[idx];
    _events[idx] = OccurrenceView(
      eventId: eventId,
      calendarId: existing.calendarId,
      summary: patch.summary ?? existing.summary,
      description: patch.description != null ? patch.description!.$1 : existing.description,
      location: patch.location != null ? patch.location!.$1 : existing.location,
      dtstart: patch.dtstart ?? existing.dtstart,
      dtend: patch.dtend ?? existing.dtend,
      isAllDay: patch.isAllDay ?? existing.isAllDay,
      timezone: patch.timezone != null ? patch.timezone!.$1 : existing.timezone,
      hasRecurrence: existing.hasRecurrence,
      recurrenceIdMs: existing.recurrenceIdMs,
      status: existing.status,
    );
  }

  @override
  Future<void> deleteEvent(
    String eventId,
    DeleteScope scope, {
    int? recurrenceIdMs,
  }) async {
    _assertInit();
    _events.removeWhere((e) => e.eventId == eventId);
  }

  @override
  Future<SyncResult> syncNow(String calendarId) async {
    _assertInit();
    await Future<void>.delayed(const Duration(milliseconds: 500));
    return const SyncResult(fetched: 0, deleted: 0, errors: []);
  }

  void _assertInit() {
    assert(_initialized, 'Bridge.init() must be called before using the bridge');
  }
}
