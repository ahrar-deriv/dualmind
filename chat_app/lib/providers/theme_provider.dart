import 'package:flutter/material.dart';

class ThemeProvider extends ChangeNotifier {
  late bool _isDarkMode;
  bool _useSystemTheme = true;

  ThemeProvider() {
    // Initialize with system theme
    _isDarkMode = WidgetsBinding.instance.platformDispatcher.platformBrightness == Brightness.dark;
    
    // Listen to system theme changes
    WidgetsBinding.instance.platformDispatcher.onPlatformBrightnessChanged = () {
      if (_useSystemTheme) {
        _isDarkMode = WidgetsBinding.instance.platformDispatcher.platformBrightness == Brightness.dark;
        notifyListeners();
      }
    };
  }

  bool get isDarkMode => _isDarkMode;
  bool get useSystemTheme => _useSystemTheme;

  ThemeData get theme => _isDarkMode ? _darkTheme : _lightTheme;

  static final _lightTheme = ThemeData(
    useMaterial3: true,
    brightness: Brightness.light,
    colorScheme: ColorScheme.fromSeed(
      seedColor: Colors.blue,
      brightness: Brightness.light,
    ),
  );

  static final _darkTheme = ThemeData(
    useMaterial3: true,
    brightness: Brightness.dark,
    colorScheme: ColorScheme.fromSeed(
      seedColor: Colors.blue,
      brightness: Brightness.dark,
    ),
  );

  void toggleTheme() {
    _useSystemTheme = false;  // Disable system theme when manually toggled
    _isDarkMode = !_isDarkMode;
    notifyListeners();
  }

  void setSystemTheme() {
    _useSystemTheme = true;
    _isDarkMode = WidgetsBinding.instance.platformDispatcher.platformBrightness == Brightness.dark;
    notifyListeners();
  }
}
