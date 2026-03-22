#ifndef AQKANJI2KOE_H
#define AQKANJI2KOE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

enum {
    AQK2K_OK = 0,
    AQK2K_ERR_INVALID_ARG = 1,
    AQK2K_ERR_NOT_INIT = 2,
    AQK2K_ERR_BUFFER_SMALL = 3,
    AQK2K_ERR_PROCESSING = 4
};

#if defined(_WIN32) && !defined(AQK2K_STATIC)
#define AQK2K_API __declspec(dllimport)
#else
#define AQK2K_API
#endif

AQK2K_API int aqk2k_create(void);
AQK2K_API void aqk2k_release(void);

AQK2K_API int aqk2k_convert(const char *input_utf8, char *out_buf, int buf_size);
AQK2K_API int aqk2k_convert_roman(const char *input_utf8, char *out_buf, int buf_size);

AQK2K_API int aqk2k_convert_u16(const uint16_t *input_utf16, char *out_buf, int buf_size);
AQK2K_API int aqk2k_convert_roman_u16(const uint16_t *input_utf16, char *out_buf, int buf_size);

#ifdef __cplusplus
}
#endif

#endif
