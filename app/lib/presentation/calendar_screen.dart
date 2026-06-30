import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../state/calendar_providers.dart';
import '../state/event_providers.dart';
import 'widgets/month_grid.dart';
import 'widgets/agenda_list.dart';
import 'event_detail_screen.dart';

class CalendarScreen extends ConsumerWidget {
  const CalendarScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final selectedDay = ref.watch(selectedDateProvider);
    final month = DateTime(selectedDay.year, selectedDay.month);
    final monthOccs = ref.watch(monthOccurrencesProvider);
    final dayOccs = ref.watch(dayOccurrencesProvider(selectedDay));

    return Scaffold(
      appBar: AppBar(
        title: Text(_monthLabel(month)),
        actions: [
          IconButton(
            icon: const Icon(Icons.today),
            tooltip: 'Go to today',
            onPressed: () {
              final today = DateTime.now();
              ref.read(selectedDateProvider.notifier).state =
                  DateTime(today.year, today.month, today.day);
            },
          ),
        ],
      ),
      body: Column(
        children: [
          monthOccs.when(
            data: (occs) => MonthGrid(
              month: month,
              occurrences: occs,
              selectedDay: selectedDay,
              onDayTap: (d) =>
                  ref.read(selectedDateProvider.notifier).state = d,
            ),
            loading: () => const LinearProgressIndicator(),
            error: (e, _) => Text('Error: $e'),
          ),
          const Divider(height: 1),
          Expanded(
            child: dayOccs.when(
              data: (occs) => AgendaList(
                occurrences: occs,
                onTap: (occ) => Navigator.of(context).push(
                  MaterialPageRoute(
                    builder: (_) => EventDetailScreen(occurrence: occ),
                  ),
                ),
              ),
              loading: () => const Center(child: CircularProgressIndicator()),
              error: (e, _) => Center(child: Text('Error: $e')),
            ),
          ),
        ],
      ),
      floatingActionButton: FloatingActionButton(
        tooltip: 'New event',
        onPressed: () => _createEvent(context, ref, selectedDay),
        child: const Icon(Icons.add),
      ),
    );
  }

  static String _monthLabel(DateTime m) {
    const months = [
      'January', 'February', 'March', 'April', 'May', 'June',
      'July', 'August', 'September', 'October', 'November', 'December',
    ];
    return '${months[m.month - 1]} ${m.year}';
  }

  Future<void> _createEvent(BuildContext context, WidgetRef ref, DateTime day) async {
    final cals = ref.read(calendarsProvider).valueOrNull ?? [];
    if (cals.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Create a calendar first')),
      );
      return;
    }
    await Navigator.of(context).push(
      MaterialPageRoute(
        builder: (_) => EventEditScreen(
          initialDate: day,
          calendarId: cals.first.id,
        ),
      ),
    );
  }
}

class EventEditScreen extends ConsumerStatefulWidget {
  const EventEditScreen({
    super.key,
    required this.initialDate,
    required this.calendarId,
    this.existing,
  });

  final DateTime initialDate;
  final String calendarId;
  final OccurrenceViewArgs? existing;

  @override
  ConsumerState<EventEditScreen> createState() => _EventEditScreenState();
}

class OccurrenceViewArgs {
  const OccurrenceViewArgs({
    required this.eventId,
    required this.summary,
    this.description,
    this.location,
    required this.dtstart,
    required this.dtend,
    required this.isAllDay,
  });
  final String eventId;
  final String summary;
  final String? description;
  final String? location;
  final DateTime dtstart;
  final DateTime dtend;
  final bool isAllDay;
}

class _EventEditScreenState extends ConsumerState<EventEditScreen> {
  late final TextEditingController _summary;
  late final TextEditingController _description;
  late DateTime _start;
  late DateTime _end;
  bool _saving = false;

  @override
  void initState() {
    super.initState();
    _summary = TextEditingController(text: widget.existing?.summary ?? '');
    _description = TextEditingController(text: widget.existing?.description ?? '');
    _start = widget.existing?.dtstart ??
        DateTime(widget.initialDate.year, widget.initialDate.month,
            widget.initialDate.day, 9, 0);
    _end = widget.existing?.dtend ?? _start.add(const Duration(hours: 1));
  }

  @override
  void dispose() {
    _summary.dispose();
    _description.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(widget.existing == null ? 'New event' : 'Edit event'),
        actions: [
          if (_saving)
            const Padding(
              padding: EdgeInsets.all(16),
              child: SizedBox(width: 20, height: 20, child: CircularProgressIndicator(strokeWidth: 2)),
            )
          else
            TextButton(onPressed: _save, child: const Text('Save')),
        ],
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          TextField(
            controller: _summary,
            decoration: const InputDecoration(labelText: 'Title', border: OutlineInputBorder()),
            autofocus: true,
          ),
          const SizedBox(height: 16),
          _DateTimeRow(
            label: 'Starts',
            value: _start,
            onChanged: (dt) => setState(() => _start = dt),
          ),
          _DateTimeRow(
            label: 'Ends',
            value: _end,
            onChanged: (dt) => setState(() => _end = dt),
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _description,
            decoration: const InputDecoration(labelText: 'Notes', border: OutlineInputBorder()),
            maxLines: 4,
          ),
        ],
      ),
    );
  }

  Future<void> _save() async {
    if (_summary.text.trim().isEmpty) return;
    setState(() => _saving = true);
    try {
      final notifier = ref.read(eventsNotifierProvider.notifier);
      if (widget.existing == null) {
        await notifier.createEvent(
          widget.calendarId,
          EventDraft(
            summary: _summary.text.trim(),
            description: _description.text.trim().isEmpty ? null : _description.text.trim(),
            dtstart: _start,
            dtend: _end,
            isAllDay: false,
          ),
        );
      } else {
        await notifier.updateEvent(
          widget.existing!.eventId,
          EventPatch(
            summary: _summary.text.trim(),
            description: (_description.text.trim().isEmpty ? null : _description.text.trim(),),
            dtstart: _start,
            dtend: _end,
          ),
        );
      }
      if (mounted) Navigator.of(context).pop();
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }
}

class _DateTimeRow extends StatelessWidget {
  const _DateTimeRow({required this.label, required this.value, required this.onChanged});

  final String label;
  final DateTime value;
  final ValueChanged<DateTime> onChanged;

  @override
  Widget build(BuildContext context) {
    return ListTile(
      title: Text(label),
      subtitle: Text(_format(value)),
      trailing: const Icon(Icons.chevron_right),
      onTap: () async {
        final date = await showDatePicker(
          context: context,
          initialDate: value,
          firstDate: DateTime(2000),
          lastDate: DateTime(2100),
        );
        if (date == null || !context.mounted) return;
        final time = await showTimePicker(
          context: context,
          initialTime: TimeOfDay.fromDateTime(value),
        );
        if (time == null) return;
        onChanged(DateTime(date.year, date.month, date.day, time.hour, time.minute));
      },
    );
  }

  static String _format(DateTime dt) {
    final months = ['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];
    return '${months[dt.month - 1]} ${dt.day}, ${dt.year}  '
        '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
  }
}

// Re-export EventDraft and EventPatch so calendar_screen.dart callers can use them
// without a second import. In the final app these come from frb generated code.
export '../bridge/bridge.dart' show EventDraft, EventPatch, OccurrenceView;
