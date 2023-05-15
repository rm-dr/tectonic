#ifndef BIBTEX_BINDINGS_H
#define BIBTEX_BINDINGS_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include "tectonic_bridge_core.h"

typedef enum {
  BUF_TY_BASE,
  BUF_TY_SV,
  BUF_TY_EX,
} BufTy;

typedef enum {
  HISTORY_SPOTLESS = 0,
  HISTORY_WARNING_ISSUED = 1,
  HISTORY_ERROR_ISSUED = 2,
  HISTORY_FATAL_ERROR = 3,
  HISTORY_ABORTED = 4,
} History;

typedef enum {
  ID_CLASS_ILLEGAL_ID_CHAR = 0,
  ID_CLASS_LEGAL_ID_CHAR = 1,
} IdClass;

typedef enum {
  LEX_CLASS_ILLEGAL = 0,
  LEX_CLASS_WHITESPACE = 1,
  LEX_CLASS_ALPHA = 2,
  LEX_CLASS_NUMERIC = 3,
  LEX_CLASS_SEP = 4,
  LEX_CLASS_OTHER = 5,
} LexClass;

typedef enum {
  SCAN_RES_ID_NULL = 0,
  SCAN_RES_SPECIFIED_CHAR_ADJACENT = 1,
  SCAN_RES_OTHER_CHAR_ADJACENT = 2,
  SCAN_RES_WHITESPACE_ADJACENT = 3,
} ScanRes;

typedef int32_t StrNumber;

typedef uint8_t ASCIICode;

typedef ASCIICode *BufType;

typedef int32_t BufPointer;

typedef int32_t CiteNumber;

typedef struct {
  int min_crossrefs;
} BibtexConfig;

typedef struct {
  ttbc_input_handle_t *handle;
  int peek_char;
  bool saw_eof;
} PeekableInput;

typedef uintptr_t PoolPointer;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

extern const LexClass LEX_CLASS[256];

extern const IdClass ID_CLASS[256];

extern const int32_t CHAR_WIDTH[256];

void reset_all(void);

bool bib_str_eq_buf(StrNumber s, BufType buf, BufPointer bf_ptr, BufPointer len);

void lower_case(BufType buf, BufPointer bf_ptr, BufPointer len);

void upper_case(BufType buf, BufPointer bf_ptr, BufPointer len);

/**
 * # Safety
 *
 * Passed pointer must point to a valid array that we have exclusive access to for the duration
 * of this call, that is at least as long as `right_end`, and initialized for the range
 * `ptr[left_end..right_end]`
 */
void quick_sort(StrNumber *cite_info, CiteNumber left_end, CiteNumber right_end);

void int_to_ascii(int32_t the_int, BufTy int_buf, BufPointer int_begin, BufPointer *int_end);

extern History tt_engine_bibtex_main(ttbc_state_t *api,
                                     const BibtexConfig *cfg,
                                     const char *aux_name);

int32_t bib_buf_size(void);

BufType bib_buf(BufTy ty);

ASCIICode bib_buf_at(BufTy ty, BufPointer num);

ASCIICode bib_buf_at_offset(BufTy ty, uintptr_t num);

BufPointer bib_buf_offset(BufTy ty, uintptr_t num);

void bib_set_buf_offset(BufTy ty, uintptr_t num, BufPointer offset);

void buffer_overflow(void);

History get_history(void);

void set_history(History hist);

void mark_warning(void);

void mark_error(void);

void mark_fatal(void);

ttbc_output_handle_t *init_log_file(const char *file);

ttbc_output_handle_t *standard_output(void);

ttbc_output_handle_t *bib_log_file(void);

void putc_log(int c);

void puts_log(const char *str);

void ttstub_puts(ttbc_output_handle_t *handle, const char *s);

void print_overflow(void);

void print_confusion(void);

void out_token(ttbc_output_handle_t *handle);

void print_a_token(void);

void print_bad_input_line(BufPointer last);

void print_skipping_whatever_remains(void);

bool out_pool_str(ttbc_output_handle_t *handle, StrNumber s);

bool print_a_pool_str(StrNumber s);

PeekableInput *peekable_open(const char *path, ttbc_file_format format);

int peekable_close(PeekableInput *peekable);

bool tectonic_eof(PeekableInput *peekable);

bool input_ln(BufPointer *last, PeekableInput *peekable);

bool str_ends_with(StrNumber s, StrNumber ext);

bool bib_str_eq_str(StrNumber s1, StrNumber s2);

void pool_overflow(void);

ASCIICode bib_str_pool(PoolPointer idx);

void bib_set_str_pool(PoolPointer idx, ASCIICode code);

PoolPointer bib_str_ptr(void);

void bib_set_str_ptr(PoolPointer ptr);

PoolPointer bib_str_start(StrNumber s);

void bib_set_str_start(StrNumber s, PoolPointer ptr);

uintptr_t bib_pool_size(void);

uintptr_t bib_max_strings(void);

bool scan1(ASCIICode char1, BufPointer last);

bool scan1_white(ASCIICode char1, BufPointer last);

bool scan2(ASCIICode char1, ASCIICode char2, BufPointer last);

bool scan2_white(ASCIICode char1, ASCIICode char2, BufPointer last);

bool scan3(ASCIICode char1, ASCIICode char2, ASCIICode char3, BufPointer last);

bool scan_alpha(BufPointer last);

bool scan_white_space(BufPointer last);

ScanRes scan_identifier(ASCIICode char1, ASCIICode char2, ASCIICode char3, BufPointer last);

bool scan_nonneg_integer(BufPointer last);

bool scan_integer(int32_t *token_value, BufPointer last);

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus

#endif /* BIBTEX_BINDINGS_H */
