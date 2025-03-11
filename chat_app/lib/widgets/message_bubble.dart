import 'package:flutter/material.dart';
import 'package:flutter_markdown/flutter_markdown.dart';
import 'package:markdown/markdown.dart' as md;
import 'package:provider/provider.dart';
import '../models/chat_message.dart';
import '../providers/theme_provider.dart';
import 'code_block.dart';

class CustomCodeBlockBuilder extends MarkdownElementBuilder {
  @override
  Widget? visitElementAfter(md.Element element, TextStyle? preferredStyle) {
    var language = '';
    
    // Check if this is a fenced code block
    if (element.attributes['class'] != null) {
      final classes = element.attributes['class']!.split(' ');
      // The language class is typically formatted as 'language-xxx'
      final languageClass = classes.firstWhere(
        (cls) => cls.startsWith('language-'),
        orElse: () => '',
      );
      if (languageClass.isNotEmpty) {
        language = languageClass.replaceFirst('language-', '');
      }
    }

    return CodeBlock(
      code: element.textContent,
      language: language.isNotEmpty ? language : null,
    );
  }
}

class MessageBubble extends StatelessWidget {
  final ChatMessage message;

  const MessageBubble({super.key, required this.message});

  @override
  Widget build(BuildContext context) {
    final isUser = message.role == 'user';
    return Consumer<ThemeProvider>(
      builder: (context, themeProvider, _) {
        final isDark = themeProvider.isDarkMode;
        return Container(
      margin: const EdgeInsets.symmetric(vertical: 8.0, horizontal: 16.0),
      child: Row(
        mainAxisAlignment: isUser ? MainAxisAlignment.end : MainAxisAlignment.start,
        children: [
          Flexible(
            child: Container(
              constraints: BoxConstraints(
                maxWidth: MediaQuery.of(context).size.width * 0.75,
              ),
              padding: const EdgeInsets.symmetric(
                horizontal: 16.0,
                vertical: 12.0,
              ),
              decoration: BoxDecoration(
                color: isUser 
                  ? (isDark ? Colors.blue[900] : Colors.blue[100])
                  : (isDark ? Colors.grey[800] : Colors.grey[200]),
                borderRadius: BorderRadius.circular(20.0),
                boxShadow: [
                  BoxShadow(
                    color: isDark 
                      ? Colors.black.withOpacity(0.3)
                      : Colors.black.withOpacity(0.1),
                    blurRadius: 2.0,
                    offset: const Offset(0, 1),
                  ),
                ],
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    message.role.toUpperCase(),
                    style: TextStyle(
                      fontSize: 12.0,
                      color: isDark ? Colors.grey[300] : Colors.grey[600],
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                  const SizedBox(height: 4.0),
                  MarkdownBody(
                    data: message.content,
                    styleSheet: MarkdownStyleSheet(
                      p: TextStyle(
                        fontSize: 16.0,
                        color: isDark ? Colors.grey[300] : Colors.grey[900],
                      ),
                      code: TextStyle(
                        fontFamily: 'monospace',
                        fontSize: 14.0,
                        color: isDark ? Colors.grey[300] : Colors.grey[900],
                      ),
                    ),
                    builders: {
                      'code': CustomCodeBlockBuilder(),
                    },
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
        });
  }
}
