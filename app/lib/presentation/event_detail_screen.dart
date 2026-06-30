import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../bridge/bridge.dart';
import '../state/event_providers.dart';
import 'calendar_screen.dart' show EventEditScreen, OccurrenceViewArgs;

class EventDetailScreen extends ConsumerWidget {
  const EventDetailScreen({super.key, required this.occurrence});

  final OccurrenceView occurrence;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Event'),
        actions: [
          IconButton(
            icon: const Icon(Icons.edit),
            onPressed: () => Navigator.of(context).push(
              MaterialPageRoute(
                builder: (_) => EventEditScreen(
                  initialDate: occurrence.dtstart,
                  calendarId: occurrence.calendarId,
                  existing: OccurrenceViewArgs(
                    eventId: occurrence.eventId,
                    summary: occurrence.summary,
                    description: occurrence.description,
                    location: occurrence.location,
                    dtstart: occurrence.dtstart,
                    dtend: occurrence.dtend,
                    isAllDay: occurrence.isAllDay,
                  ),
                ),
              ),
            ),
          ),
          IconButton(
            icon: const Icon(Icons.delete_outline),
            onPressed: () => _confirmDelete(context, ref),
          ),
        ],
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Text(occurrence.summary, style: Theme.of(context).textTheme.headlineSmall),
          const SizedBox(height: 8),
          _InfoRow(
            icon: Icons.access_time,
            text: occurrence.isAllDay
                ? _dateLabel(occurrence.dtstart)
                : '${_dateTimeLabel(occurrence.dtstart)} – ${_timeLabel(occurrence.dtend)}',
          ),
          if (occurrence.hasRecurrence)
            const _InfoRow(icon: Icons.repeat, text: 'Recurring event'),
          if (occurrence.location != null)
            _InfoRow(icon: Icons.place, text: occurrence.location!),
          if (occurrence.description != null) ...[
            const SizedBox(height: 16),
            Text(occurrence.description!, style: Theme.of(context).textTheme.bodyMedium),
          ],
        ],
      ),
    );
  }

  Future<void> _confirmDelete(BuildContext context, WidgetRef ref) async {
    final scope = occurrence.hasRecurrence
        ? await _showRecurrenceDeleteDialog(context)
        : DeleteScope.all;
    if (scope == null || !context.mounted) return;

    await ref.read(eventsNotifierProvider.notifier).deleteEvent(occurrence.eventId, scope);
    if (context.mounted) Navigator.of(context).pop();
  }

  Future<DeleteScope?> _showRecurrenceDeleteDialog(BuildContext context) {
    return showDialog<DeleteScope>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Delete recurring event'),
        content: const Text('Which occurrences do you want to delete?'),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(DeleteScope.thisOnly),
            child: const Text('This event'),
          ),
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(DeleteScope.thisAndFuture),
            child: const Text('This and future'),
          ),
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(DeleteScope.all),
            child: const Text('All events'),
          ),
        ],
      ),
    );
  }

  static String _dateLabel(DateTime dt) {
    const months = ['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];
    return '${months[dt.month - 1]} ${dt.day}, ${dt.year}';
  }

  static String _timeLabel(DateTime dt) =>
      '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';

  static String _dateTimeLabel(DateTime dt) => '${_dateLabel(dt)}  ${_timeLabel(dt)}';
}

class _InfoRow extends StatelessWidget {
  const _InfoRow({required this.icon, required this.text});
  final IconData icon;
  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(icon, size: 20, color: Theme.of(context).colorScheme.outline),
          const SizedBox(width: 12),
          Expanded(child: Text(text)),
        ],
      ),
    );
  }
}
