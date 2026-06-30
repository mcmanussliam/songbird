import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../state/calendar_providers.dart';
import 'add_caldav_screen.dart';

class CalendarListScreen extends ConsumerWidget {
  const CalendarListScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final cals = ref.watch(calendarsProvider);

    return Scaffold(
      appBar: AppBar(title: const Text('Calendars')),
      body: cals.when(
        data: (list) => ListView(
          children: [
            ...list.map((cal) => ListTile(
              leading: Icon(
                cal.isCalDav ? Icons.cloud_outlined : Icons.calendar_today,
              ),
              title: Text(cal.displayName),
              subtitle: Text(cal.source),
              trailing: cal.isCalDav
                  ? IconButton(
                      icon: const Icon(Icons.sync),
                      onPressed: () => _sync(context, ref, cal.id),
                    )
                  : null,
            )),
            const Divider(),
            ListTile(
              leading: const Icon(Icons.add),
              title: const Text('New local calendar'),
              onTap: () => _createLocal(context, ref),
            ),
            ListTile(
              leading: const Icon(Icons.cloud_upload_outlined),
              title: const Text('Add CalDAV account'),
              onTap: () => Navigator.of(context).push(
                MaterialPageRoute(builder: (_) => const AddCalDavScreen()),
              ),
            ),
          ],
        ),
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(child: Text('Error: $e')),
      ),
    );
  }

  Future<void> _createLocal(BuildContext context, WidgetRef ref) async {
    final name = await _promptName(context, 'New calendar');
    if (name == null || !context.mounted) return;
    await ref.read(calendarsProvider.notifier).createLocal(name);
  }

  Future<void> _sync(BuildContext context, WidgetRef ref, String calendarId) async {
    final result = await ref.read(calendarsProvider.notifier).syncCalendar(calendarId);
    if (!context.mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          result.errors.isEmpty
              ? 'Synced: ${result.fetched} fetched, ${result.deleted} deleted'
              : 'Sync completed with ${result.errors.length} error(s)',
        ),
      ),
    );
  }

  Future<String?> _promptName(BuildContext context, String hint) {
    final controller = TextEditingController();
    return showDialog<String>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Calendar name'),
        content: TextField(
          controller: controller,
          decoration: InputDecoration(hintText: hint),
          autofocus: true,
          onSubmitted: (v) => Navigator.of(ctx).pop(v.trim()),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(),
            child: const Text('Cancel'),
          ),
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(controller.text.trim()),
            child: const Text('Create'),
          ),
        ],
      ),
    );
  }
}
