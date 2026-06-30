import 'package:flutter_local_notifications/flutter_local_notifications.dart';

final _plugin = FlutterLocalNotificationsPlugin();

Future<void> initNotifications() async {
  const android = AndroidInitializationSettings('@mipmap/ic_launcher');
  const ios = DarwinInitializationSettings(requestAlertPermission: false);
  await _plugin.initialize(
    const InitializationSettings(android: android, iOS: ios),
  );
}

Future<void> scheduleEventReminder({
  required int id,
  required String title,
  required String body,
  required DateTime scheduledAt,
}) async {
  await _plugin.show(
    id,
    title,
    body,
    const NotificationDetails(
      android: AndroidNotificationDetails(
        'songbird_reminders',
        'Event reminders',
        channelDescription: 'On-device reminders for calendar events',
        importance: Importance.high,
        priority: Priority.high,
      ),
      iOS: DarwinNotificationDetails(
        presentAlert: true,
        presentBadge: true,
        presentSound: true,
      ),
    ),
  );
}

Future<void> cancelReminder(int id) => _plugin.cancel(id);
