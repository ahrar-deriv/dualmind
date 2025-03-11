import 'package:flutter_dotenv/flutter_dotenv.dart';

class EnvConfig {
  static String get apiUrl => dotenv.get('API_URL', fallback: '');
  static String get apiKey => dotenv.get('API_KEY', fallback: '');
  
  static Future<void> load() async {
    await dotenv.load(fileName: ".env");
  }
}
