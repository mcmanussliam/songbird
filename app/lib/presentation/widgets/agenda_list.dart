import 'package:flutter/material.dart';
import '../../bridge/bridge.dart';

class AgendaList extends StatelessWidget {
  const AgendaList({
    super.key,
    required this.occurrences,
    required this.onTap,
  });

  final List<OccurrenceView> occurrences;
  final ValueChanged<OccurrenceView> onTap;

  @override
  Widget build(BuildContext context) {
    if (occurrences.isEmpty) {
      return const Center(
        child: Text('No events', style: TextStyle(color: Colors.grey)),
      );
    }

    return ListView.builder(
      itemCount: occurrences.length,
      itemBuilder: (context, i) {
        final occ = occurrences[i];
        return _OccurrenceTile(occ: occ, onTap: () => onTap(occ));
      },
    );
  }
}

class _OccurrenceTile extends StatelessWidget {
  const _OccurrenceTile({required this.occ, required this.onTap});

  final OccurrenceView occ;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final timeLabel = occ.isAllDay
        ? 'All day'
        : '${_hm(occ.dtstart)} - ${_hm(occ.dtend)}';

    return ListTile(
      onTap: onTap,
      leading: Container(
        width: 4,
        height: 40,
        decoration: BoxDecoration(
          color: theme.colorScheme.primary,
          borderRadius: BorderRadius.circular(2),
        ),
      ),
      title: Text(occ.summary),
      subtitle: Text(timeLabel),
      trailing: occ.hasRecurrence
          ? Icon(Icons.repeat, size: 16, color: theme.colorScheme.outline)
          : null,
    );
  }

  static String _hm(DateTime dt) =>
      '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
}
