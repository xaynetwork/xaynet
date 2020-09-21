#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

#include "minunit.h"
#include "xaynet_ffi.h"

int sum_array(unsigned char *array, int len)
{
  int sum = 0;
  for (int i = 0; i < len; i++)
  {
    sum = sum + array[i];
  }
  return sum;
}

static char *test_xaynet_ffi_new_secret_key()
{
  int secret_key_length = 64;
  unsigned char secret_key[secret_key_length];
  memset(secret_key, 0, secret_key_length);
  mu_assert("error, c_buffer[0] != 0", (sum_array(secret_key, secret_key_length) == 0));
  xaynet_ffi_new_secret_key(secret_key);
  mu_assert("error, c_buffer[0] != 0", (sum_array(secret_key, secret_key_length) != 0));
  return 0;
}

static char *test_xaynet_ffi_init()
{
  unsigned char secret_key[64] = {0};
  xaynet_ffi_new_secret_key(secret_key);
  char *url = "http://localhost:8081";

  CMobileClient *client = xaynet_ffi_init_mobile_client(url, secret_key, 0, 0, 0, 3, 1);
  mu_assert("error, client == null", client != NULL);

  xaynet_ffi_destroy_mobile_client(client);
  return 0;
}

static char *test_xaynet_ffi_init_wrong_group_type()
{
  unsigned char secret_key[64] = {0};
  xaynet_ffi_new_secret_key(secret_key);
  char *url = "http://localhost:8081";

  CMobileClient *client = xaynet_ffi_init_mobile_client(url, secret_key, 0, 12, 0, 3, 1);
  mu_assert("error, client == null", client == NULL);
  return 0;
}

static char *test_xaynet_ffi_serialize()
{
  unsigned char secret_key[64] = {0};
  xaynet_ffi_new_secret_key(secret_key);
  char *url = "http://localhost:8081";

  CMobileClient *client = xaynet_ffi_init_mobile_client(url, secret_key, 0, 0, 0, 3, 1);

  BytesBuffer *buffer = xaynet_ffi_serialize_mobile_client(client);
  mu_assert("error, byte buffer == null", client != NULL);

  unsigned int len_buffer = xaynet_ffi_get_len_of_byte_buffer(buffer);
  mu_assert("error, byte buffer len <= 0", len_buffer > 0);

  unsigned char c_buffer[len_buffer];
  memset(c_buffer, 0, len_buffer);
  mu_assert("error, c_buffer[0] != 0", (sum_array(c_buffer, len_buffer) == 0));

  xaynet_ffi_copy_into_foreign_buffer(buffer, c_buffer);
  mu_assert("error, c_buffer[0] == 0", (sum_array(c_buffer, len_buffer) != 0));

  xaynet_ffi_destroy_byte_buffer(buffer);
  xaynet_ffi_destroy_mobile_client(client);
  return 0;
}

static char *test_xaynet_ffi_restore()
{
  unsigned char secret_key[64] = {0};
  xaynet_ffi_new_secret_key(secret_key);
  char *url = "http://localhost:8081";

  CMobileClient *client = xaynet_ffi_init_mobile_client(url, secret_key, 0, 0, 0, 3, 1);

  BytesBuffer *buffer = xaynet_ffi_serialize_mobile_client(client);
  unsigned int size_buffer = xaynet_ffi_get_len_of_byte_buffer(buffer);

  unsigned char c_buffer[size_buffer];
  memset(c_buffer, 0, size_buffer);

  xaynet_ffi_copy_into_foreign_buffer(buffer, c_buffer);
  xaynet_ffi_destroy_mobile_client(client);

  CMobileClient *de_client = xaynet_ffi_restore_mobile_client(url, c_buffer, size_buffer);
  mu_assert("error, client == null", de_client != NULL);

  xaynet_ffi_destroy_byte_buffer(buffer);
  xaynet_ffi_destroy_mobile_client(de_client);
  return 0;
}

static char *test_xaynet_ffi_try_to_proceed_mobile_client()
{
  unsigned char secret_key[64];
  xaynet_ffi_new_secret_key(secret_key);
  char *url = "http://localhost:8081";

  CMobileClient *client = xaynet_ffi_init_mobile_client(url, secret_key, 0, 0, 0, 3, 1);
  mu_assert("error, client == null", client != NULL);

  CMobileClient *next_client = xaynet_ffi_try_to_proceed_mobile_client(client);
  mu_assert("error, new_client == null", next_client != NULL);
  mu_assert("error, client == next_client", client != next_client);

  xaynet_ffi_destroy_mobile_client(next_client);
  return 0;
}

static char *all_tests()
{
  mu_run_test(test_xaynet_ffi_new_secret_key);
  mu_run_test(test_xaynet_ffi_init);
  mu_run_test(test_xaynet_ffi_init_wrong_group_type);
  mu_run_test(test_xaynet_ffi_serialize);
  mu_run_test(test_xaynet_ffi_restore);
  mu_run_test(test_xaynet_ffi_try_to_proceed_mobile_client);

  return 0;
}

int tests_run = 0;

int main(int argc, char **argv)
{
  char *result = all_tests();
  if (result != 0)
  {
    printf("%s\n", result);
  }
  else
  {
    printf("ALL TESTS PASSED\n");
  }
  printf("Tests run: %d\n", tests_run);

  return result != 0;
}
