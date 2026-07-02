import 'package:flutter/material.dart' show ThemeExtension;
import 'package:flutter/widgets.dart';

class AppColors {
  const AppColors._();

  static const Color _accent = Color(0xFF5B6CFF);
  static const Color _accentDark = Color(0xFF8B97FF);

  static const Color _success = Color(0xFF2FB380);
  static const Color _warning = Color(0xFFDB9A2C);
  static const Color _danger = Color(0xFFE0555A);

  static const light = AppColorScheme(
    background: Color(0xFFFBFBFA),
    surface: Color(0xFFFFFFFF),
    surfaceRaised: Color(0xFFF3F3F1),
    border: Color(0xFFE7E7E4),
    textPrimary: Color(0xFF1B1B18),
    textSecondary: Color(0xFF6E6E68),
    textTertiary: Color(0xFFA3A39B),
    accent: _accent,
    onAccent: Color(0xFFFFFFFF),
    success: _success,
    warning: _warning,
    danger: _danger,
  );

  static const dark = AppColorScheme(
    background: Color(0xFF121212),
    surface: Color(0xFF1B1B1B),
    surfaceRaised: Color(0xFF242424),
    border: Color(0xFF303030),
    textPrimary: Color(0xFFF2F2F0),
    textSecondary: Color(0xFFA8A8A2),
    textTertiary: Color(0xFF6E6E68),
    accent: _accentDark,
    onAccent: Color(0xFF17193A),
    success: _success,
    warning: _warning,
    danger: _danger,
  );

  static const calendarPalette = <Color>[
    Color(0xFF5B6CFF),
    Color(0xFF2FB380),
    Color(0xFFDB9A2C),
    Color(0xFFE0555A),
    Color(0xFF2C9BDB),
    Color(0xFFB25BD8),
  ];
}

class AppColorScheme extends ThemeExtension<AppColorScheme> {
  const AppColorScheme({
    required this.background,
    required this.surface,
    required this.surfaceRaised,
    required this.border,
    required this.textPrimary,
    required this.textSecondary,
    required this.textTertiary,
    required this.accent,
    required this.onAccent,
    required this.success,
    required this.warning,
    required this.danger,
  });

  final Color background;
  final Color surface;
  final Color surfaceRaised;
  final Color border;
  final Color textPrimary;
  final Color textSecondary;
  final Color textTertiary;
  final Color accent;
  final Color onAccent;
  final Color success;
  final Color warning;
  final Color danger;

  @override
  AppColorScheme copyWith({
    Color? background,
    Color? surface,
    Color? surfaceRaised,
    Color? border,
    Color? textPrimary,
    Color? textSecondary,
    Color? textTertiary,
    Color? accent,
    Color? onAccent,
    Color? success,
    Color? warning,
    Color? danger,
  }) {
    return AppColorScheme(
      background: background ?? this.background,
      surface: surface ?? this.surface,
      surfaceRaised: surfaceRaised ?? this.surfaceRaised,
      border: border ?? this.border,
      textPrimary: textPrimary ?? this.textPrimary,
      textSecondary: textSecondary ?? this.textSecondary,
      textTertiary: textTertiary ?? this.textTertiary,
      accent: accent ?? this.accent,
      onAccent: onAccent ?? this.onAccent,
      success: success ?? this.success,
      warning: warning ?? this.warning,
      danger: danger ?? this.danger,
    );
  }

  @override
  AppColorScheme lerp(ThemeExtension<AppColorScheme>? other, double t) {
    if (other is! AppColorScheme) {
      return this;
    }

    return AppColorScheme(
      background: Color.lerp(background, other.background, t)!,
      surface: Color.lerp(surface, other.surface, t)!,
      surfaceRaised: Color.lerp(surfaceRaised, other.surfaceRaised, t)!,
      border: Color.lerp(border, other.border, t)!,
      textPrimary: Color.lerp(textPrimary, other.textPrimary, t)!,
      textSecondary: Color.lerp(textSecondary, other.textSecondary, t)!,
      textTertiary: Color.lerp(textTertiary, other.textTertiary, t)!,
      accent: Color.lerp(accent, other.accent, t)!,
      onAccent: Color.lerp(onAccent, other.onAccent, t)!,
      success: Color.lerp(success, other.success, t)!,
      warning: Color.lerp(warning, other.warning, t)!,
      danger: Color.lerp(danger, other.danger, t)!,
    );
  }
}
