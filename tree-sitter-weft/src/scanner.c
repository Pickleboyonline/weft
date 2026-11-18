// External scanner for weft.
//
// weft ends a statement at a line break, unless the statement obviously
// continues (reference: Lexical structure, "Lines and blocks"). The parser
// tells us when a line break could end something (valid_symbols[NEWLINE]);
// where it couldn't (after an operator, a comma or an open bracket), the break
// is ordinary whitespace. When it could, the break still doesn't end the
// statement if the next line starts with `.` (a method chain) or `else`.
//
// It also scans a `style { … }` block's CSS as one token, counting braces, and
// nested block comments.

#include "tree_sitter/parser.h"

#include <stdbool.h>
#include <stdint.h>

enum TokenType { NEWLINE, CSS_BODY, BLOCK_COMMENT };

void *tree_sitter_weft_external_scanner_create(void) { return NULL; }
void tree_sitter_weft_external_scanner_destroy(void *payload) {}
unsigned tree_sitter_weft_external_scanner_serialize(void *payload, char *buffer) { return 0; }
void tree_sitter_weft_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {}

static void advance(TSLexer *lexer) { lexer->advance(lexer, false); }
static void skip(TSLexer *lexer) { lexer->advance(lexer, true); }

static bool is_ident_char(int32_t c) {
  return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_';
}

static bool scan_css(TSLexer *lexer) {
  int depth = 0;
  bool any = false;
  while (!lexer->eof(lexer)) {
    int32_t c = lexer->lookahead;
    if (c == '{') depth++;
    if (c == '}') {
      if (depth == 0) break;
      depth--;
    }
    advance(lexer);
    any = true;
  }
  if (!any) return false;
  lexer->mark_end(lexer);
  lexer->result_symbol = CSS_BODY;
  return true;
}

// `/* … */`, nesting.
static bool scan_block_comment(TSLexer *lexer) {
  if (lexer->lookahead != '/') return false;
  advance(lexer);
  if (lexer->lookahead != '*') return false;
  advance(lexer);
  int depth = 1;
  while (depth > 0) {
    if (lexer->eof(lexer)) return false;
    int32_t c = lexer->lookahead;
    advance(lexer);
    if (c == '/' && lexer->lookahead == '*') {
      advance(lexer);
      depth++;
    } else if (c == '*' && lexer->lookahead == '/') {
      advance(lexer);
      depth--;
    }
  }
  lexer->mark_end(lexer);
  lexer->result_symbol = BLOCK_COMMENT;
  return true;
}

static bool scan_newline(TSLexer *lexer) {
  // The token is the line break plus any blank lines after it.
  advance(lexer);
  lexer->mark_end(lexer);
  for (;;) {
    while (lexer->lookahead == ' ' || lexer->lookahead == '\t' || lexer->lookahead == '\r') advance(lexer);
    if (lexer->lookahead != '\n') break;
    advance(lexer);
    lexer->mark_end(lexer);
  }
  // Peek at the next line. Characters read past mark_end aren't part of the
  // token.
  int32_t c = lexer->lookahead;
  if (c == '.') {
    advance(lexer);
    if (lexer->lookahead != '.') return false;  // `.method` continues; `..` doesn't
  } else if (c == 'e') {
    const char *word = "else";
    int i = 0;
    while (word[i] && lexer->lookahead == word[i]) {
      advance(lexer);
      i++;
    }
    if (word[i] == 0 && !is_ident_char(lexer->lookahead)) return false;
  }
  lexer->result_symbol = NEWLINE;
  return true;
}

bool tree_sitter_weft_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid_symbols) {
  // During error recovery every symbol is valid; let the internal lexer work.
  if (valid_symbols[NEWLINE] && valid_symbols[CSS_BODY]) return false;

  if (valid_symbols[CSS_BODY]) return scan_css(lexer);

  while (lexer->lookahead == ' ' || lexer->lookahead == '\t' || lexer->lookahead == '\r') skip(lexer);

  if (valid_symbols[NEWLINE] && lexer->lookahead == '\n') return scan_newline(lexer);

  if (valid_symbols[BLOCK_COMMENT] && lexer->lookahead == '/') return scan_block_comment(lexer);

  return false;
}
