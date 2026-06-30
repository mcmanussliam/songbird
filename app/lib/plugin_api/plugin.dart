// Plugin extension API — see system-design.md §13.
//
// Designed in Phase 1, implemented/consumed starting Phase 3 (M7). The interfaces exist now
// so later plugins don't require a presentation-layer refactor.

import 'package:flutter/widgets.dart';

abstract class CalendarPlugin {
  String get id;
  String get displayName;

  /// Called whenever a new event is created, before the create is finalized.
  Future<void> onEventCreated(/* EventView event */) async {}

  /// Renders an optional card in the agenda view, per event.
  Widget? provideAgendaCard(BuildContext context /*, EventView event */) => null;

  /// Renders an optional panel in the event detail sidebar.
  Widget? provideSidebarPanel(BuildContext context /*, EventView event */) => null;
}
