/// Real bridge backed by the generated flutter_rust_bridge bindings.
///
/// Converts between the app-facing [Bridge] interface and the
/// generated Rust API surface, and loads the native
/// library on first use.
library;

import '../bridge_generated/api.dart' as frb;
import '../bridge_generated/frb_generated.dart';
import 'bridge.dart';

class BridgeFrb implements Bridge {
  @override
  Future<void> init(String dbPath) async {
    if (!RustLib.instance.initialized) {
      await RustLib.init();
    }

    await frb.init(dbPath: dbPath);
  }

  @override
  Future<List<CalendarView>> listCalendars() async {
    final calendars = await frb.listCalendars();
    return calendars.map(_toCalendarView).toList();
  }

  @override
  Future<String> createLocalCalendar(String displayName) {
    return frb.createLocalCalendar(displayName: displayName);
  }

  @override
  Future<List<String>> addCalDavAccount({
    required String baseUrl,
    required String username,
    required String password,
  }) {
    return frb.addCaldavAccount(
        baseUrl: baseUrl, username: username, password: password);
  }

  @override
  Future<List<OccurrenceView>> occurrencesInRange({
    required List<String> calendarIds,
    required DateTime start,
    required DateTime end,
  }) async {
    final occurrences = await frb.occurrencesInRange(
      calendarIds: calendarIds,
      range: frb.DateRangeMs(
        startMs: start.toUtc().millisecondsSinceEpoch,
        endMs: end.toUtc().millisecondsSinceEpoch,
      ),
    );

    return occurrences.map(_toOccurrenceView).toList();
  }

  @override
  Future<String> createEvent(String calendarId, EventDraft draft) {
    return frb.createEvent(
      calendarId: calendarId,
      draft: frb.EventDraft(
        summary: draft.summary,
        description: draft.description,
        location: draft.location,
        dtstartMs: draft.dtstart.toUtc().millisecondsSinceEpoch,
        dtendMs: draft.dtend.toUtc().millisecondsSinceEpoch,
        isAllDay: draft.isAllDay,
        timezone: draft.timezone,
        rrule: draft.rrule,
      ),
    );
  }

  @override
  Future<void> updateEvent(String eventId, EventPatch patch) {
    return frb.updateEvent(
      eventId: eventId,
      patch: frb.EventPatch(
        summary: patch.summary,
        description: _nullableUpdate(patch.description),
        location: _nullableUpdate(patch.location),
        dtstartMs: patch.dtstart?.toUtc().millisecondsSinceEpoch,
        dtendMs: patch.dtend?.toUtc().millisecondsSinceEpoch,
        isAllDay: patch.isAllDay,
        timezone: _nullableUpdate(patch.timezone),
        rrule: _nullableUpdate(patch.rrule),
      ),
    );
  }

  @override
  Future<void> deleteEvent(
    String eventId,
    DeleteScope scope, {
    int? recurrenceIdMs,
  }) {
    return frb.deleteEvent(
      eventId: eventId,
      scope: _toFrbDeleteScope(scope),
      recurrenceIdMs: recurrenceIdMs,
    );
  }

  @override
  Future<SyncResult> syncNow(String calendarId) async {
    final result = await frb.syncNow(calendarId: calendarId);
    return SyncResult(
        fetched: result.fetched,
        deleted: result.deleted,
        errors: result.errors);
  }

  frb.NullableStringUpdate? _nullableUpdate((String?,)? update) {
    if (update == null) {
      return null;
    }

    return frb.NullableStringUpdate(value: update.$1);
  }

  frb.DeleteScope _toFrbDeleteScope(DeleteScope scope) {
    switch (scope) {
      case DeleteScope.thisOnly:
        return frb.DeleteScope.thisOnly;
      case DeleteScope.thisAndFuture:
        return frb.DeleteScope.thisAndFuture;
      case DeleteScope.all:
        return frb.DeleteScope.all;
    }
  }

  CalendarView _toCalendarView(frb.CalendarView c) {
    return CalendarView(
      id: c.id,
      displayName: c.displayName,
      source: c.source,
      lastSyncedAt: c.lastSyncedMs != null
          ? DateTime.fromMillisecondsSinceEpoch(c.lastSyncedMs!, isUtc: true)
          : null,
    );
  }

  OccurrenceView _toOccurrenceView(frb.OccurrenceView o) {
    return OccurrenceView(
      eventId: o.eventId,
      calendarId: o.calendarId,
      summary: o.summary,
      description: o.description,
      location: o.location,
      dtstart: DateTime.fromMillisecondsSinceEpoch(o.dtstartMs, isUtc: true),
      dtend: DateTime.fromMillisecondsSinceEpoch(o.dtendMs, isUtc: true),
      isAllDay: o.isAllDay,
      timezone: o.timezone,
      hasRecurrence: o.hasRecurrence,
      recurrenceIdMs: o.recurrenceIdMs,
      status: o.status,
    );
  }
}
