import 'dart:convert';
import 'package:http/http.dart' as http;
import '../models/chat_message.dart';
import '../config/env_config.dart';

class ChatApiException implements Exception {
  final String message;
  final int? statusCode;

  ChatApiException(this.message, {this.statusCode});

  @override
  String toString() => 'ChatApiException: $message${statusCode != null ? ' (Status: $statusCode)' : ''}';
}

class ChatApi {
  final String _baseUrl;
  final String _apiKey;
  String? _sessionId;

  ChatApi({String? baseUrl, String? apiKey})
      : _baseUrl = baseUrl ?? EnvConfig.apiUrl,
        _apiKey = apiKey ?? EnvConfig.apiKey;

  Map<String, String> get _headers {
    final headers = {
      'Content-Type': 'application/json',
      'Authorization': 'Bearer $_apiKey',
    };
    if (_sessionId != null) {
      headers['X-Session-ID'] = _sessionId!;
    }
    return headers;
  }

  Future<Map<String, dynamic>> sendMessage(List<ChatMessage> messages) async {
    try {
      final response = await http.post(
        Uri.parse('$_baseUrl/v1/chat/completions'),
        headers: _headers,
        body: jsonEncode({
          'model': 'r1sonnet',
          'messages': messages.map((msg) => msg.toJson()).toList(),
          'stream': false,
        }),
      );

      if (response.statusCode == 200) {
        // Update session ID if present
        if (response.headers.containsKey('x-session-id')) {
          _sessionId = response.headers['x-session-id'];
        }

        final responseData = jsonDecode(response.body);
        return {
          'content': responseData['choices'][0]['message']['content'],
          'sessionId': _sessionId,
        };
      } else {
        throw ChatApiException(
          'Server returned ${response.statusCode}',
          statusCode: response.statusCode,
        );
      }
    } catch (e) {
      if (e is ChatApiException) rethrow;
      throw ChatApiException('Network error: $e');
    }
  }

  void clearSession() {
    _sessionId = null;
  }
}
