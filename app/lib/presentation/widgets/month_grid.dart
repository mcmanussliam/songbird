import 'package:flutter/material.dart';
import '../../bridge/bridge.dart';

class MonthGrid extends StatelessWidget {
  const MonthGrid({
    super.key,
    required this.month,
    required this.occurrences,
    required this.selectedDay,
    required this.onDayTap,
  });

  final DateTime month;
  final List<OccurrenceView> occurrences;
  final DateTime selectedDay;
  final ValueChanged<DateTime> onDayTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    final firstOfMonth = DateTime(month.year, month.month, 1);
    // Monday-based grid: weekday 1=Mon, 7=Sun → offset = (weekday - 1) % 7
    final startOffset = (firstOfMonth.weekday - 1) % 7;
    final daysInMonth = DateUtils.getDaysInMonth(month.year, month.month);
    final totalCells = startOffset + daysInMonth;
    final rows = (totalCells / 7).ceil();

    final eventsByDay = <int, int>{};
    for (final occ in occurrences) {
      if (occ.dtstart.year == month.year && occ.dtstart.month == month.month) {
        final d = occ.dtstart.day;
        eventsByDay[d] = (eventsByDay[d] ?? 0) + 1;
      }
    }

    final today = DateTime.now();
    final isCurrentMonth = month.year == today.year && month.month == today.month;

    return Column(
      children: [
        _WeekdayHeader(colorScheme: colorScheme),
        ...List.generate(rows, (row) {
          return Row(
            children: List.generate(7, (col) {
              final cell = row * 7 + col;
              final day = cell - startOffset + 1;
              if (day < 1 || day > daysInMonth) {
                return const Expanded(child: SizedBox(height: 44));
              }
              final date = DateTime(month.year, month.month, day);
              final isToday = isCurrentMonth && day == today.day;
              final isSelected =
                  selectedDay.year == month.year &&
                  selectedDay.month == month.month &&
                  selectedDay.day == day;
              final dotCount = eventsByDay[day] ?? 0;

              return Expanded(
                child: GestureDetector(
                  onTap: () => onDayTap(date),
                  child: _DayCell(
                    day: day,
                    isToday: isToday,
                    isSelected: isSelected,
                    dotCount: dotCount,
                    colorScheme: colorScheme,
                  ),
                ),
              );
            }),
          );
        }),
      ],
    );
  }
}

class _WeekdayHeader extends StatelessWidget {
  const _WeekdayHeader({required this.colorScheme});
  final ColorScheme colorScheme;
  static const _labels = ['M', 'T', 'W', 'T', 'F', 'S', 'S'];

  @override
  Widget build(BuildContext context) {
    return Row(
      children: _labels
          .map((l) => Expanded(
                child: Center(
                  child: Text(
                    l,
                    style: TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.w600,
                      color: colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
              ))
          .toList(),
    );
  }
}

class _DayCell extends StatelessWidget {
  const _DayCell({
    required this.day,
    required this.isToday,
    required this.isSelected,
    required this.dotCount,
    required this.colorScheme,
  });

  final int day;
  final bool isToday;
  final bool isSelected;
  final int dotCount;
  final ColorScheme colorScheme;

  @override
  Widget build(BuildContext context) {
    final bgColor = isSelected
        ? colorScheme.primary
        : isToday
            ? colorScheme.primaryContainer
            : Colors.transparent;

    final textColor = isSelected
        ? colorScheme.onPrimary
        : isToday
            ? colorScheme.onPrimaryContainer
            : colorScheme.onSurface;

    return SizedBox(
      height: 44,
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Container(
            width: 32,
            height: 32,
            decoration: BoxDecoration(color: bgColor, shape: BoxShape.circle),
            alignment: Alignment.center,
            child: Text(
              '$day',
              style: TextStyle(
                fontSize: 14,
                fontWeight: isToday || isSelected ? FontWeight.w700 : FontWeight.normal,
                color: textColor,
              ),
            ),
          ),
          if (dotCount > 0)
            Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: List.generate(
                dotCount.clamp(0, 3),
                (_) => Container(
                  width: 4,
                  height: 4,
                  margin: const EdgeInsets.symmetric(horizontal: 1),
                  decoration: BoxDecoration(
                    color: isSelected ? colorScheme.onPrimary : colorScheme.primary,
                    shape: BoxShape.circle,
                  ),
                ),
              ),
            )
          else
            const SizedBox(height: 5),
        ],
      ),
    );
  }
}
