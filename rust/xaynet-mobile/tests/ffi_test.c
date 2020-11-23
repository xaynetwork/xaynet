#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "minunit.h"
#include "xaynet_ffi.h"

static char *test_settings_new() {
  Settings *settings = xaynet_ffi_settings_new();
  xaynet_ffi_settings_destroy(settings);
  return 0;
}

static char *test_settings_set_keys() {
  mu_assert("failed to init crypto", xaynet_ffi_crypto_init() == OK);
  Settings *settings = xaynet_ffi_settings_new();
  const KeyPair *keys = xaynet_ffi_generate_key_pair();
  int err = xaynet_ffi_settings_set_keys(settings, keys);
  mu_assert("failed to set keys", !err);
  xaynet_ffi_forget_key_pair(keys);

  xaynet_ffi_settings_destroy(settings);
  return 0;
}

static char *test_settings_set_url() {
  Settings *settings = xaynet_ffi_settings_new();

  int err = xaynet_ffi_settings_set_url(settings, NULL);
  mu_assert("settings invalid URL should fail", err == ERR_INVALID_URL);

  char *url = "http://localhost:1234";
  err = xaynet_ffi_settings_set_url(settings, url);
  mu_assert("failed to set url", !err);

  char *url2 = strdup(url);
  err = xaynet_ffi_settings_set_url(settings, url2);
  mu_assert("failed to set url from allocated string", !err);

  // cleanup
  free(url2);
  xaynet_ffi_settings_destroy(settings);

  return 0;
}

void with_keys(Settings *settings) {
  const KeyPair *keys = xaynet_ffi_generate_key_pair();
  int err = xaynet_ffi_settings_set_keys(settings, keys);
  assert(!err);
  xaynet_ffi_forget_key_pair(keys);
}

void with_url(Settings *settings) {
  int err = xaynet_ffi_settings_set_url(settings, "http://localhost:1234");
  assert(!err);
}

static char *test_settings() {
  Settings *settings = xaynet_ffi_settings_new();
  with_keys(settings);
  int err = xaynet_ffi_check_settings(settings);
  mu_assert("expected missing url error", err == ERR_SETTINGS_URL);
  xaynet_ffi_settings_destroy(settings);

  settings = xaynet_ffi_settings_new();
  with_url(settings);
  err = xaynet_ffi_check_settings(settings);
  mu_assert("expected missing keys error", err == ERR_SETTINGS_KEYS);
  xaynet_ffi_settings_destroy(settings);

  return 0;
}

static char *test_participant_save_and_restore() {
  Settings *settings = xaynet_ffi_settings_new();
  with_keys(settings);
  with_url(settings);

  Participant *participant = xaynet_ffi_participant_new(settings);
  mu_assert("failed to create participant", participant != NULL);
  xaynet_ffi_settings_destroy(settings);

  // save the participant
  const ByteBuffer *save_buf = xaynet_ffi_participant_save(participant);
  mu_assert("failed to save participant", save_buf != NULL);

  // write the serialized participant to a file
  char *path = "./test_participant_save_and_restore.txt";
  FILE *f = fopen(path, "w");
  fwrite(save_buf->data, 1, save_buf->len, f);
  fclose(f);
  int err = xaynet_ffi_byte_buffer_destroy(save_buf);
  assert(!err);

  // read the serialized participant from the file
  f = fopen(path, "r");
  fseek(f, 0L, SEEK_END);
  int fsize = ftell(f);
  fseek(f, 0L, SEEK_SET);
  ByteBuffer restore_buf = {
      .len = fsize,
      .data = (uint8_t *)malloc(fsize),
  };
  int n_read = fread(restore_buf.data, 1, fsize, f);
  mu_assert("failed to read serialized participant", n_read == fsize);
  fclose(f);

  // restore the participant
  Participant *restored =
      xaynet_ffi_participant_restore("http://localhost:8081", restore_buf);
  mu_assert("failed to restore participant", restored != NULL);

  // free memory
  free(restore_buf.data);
  xaynet_ffi_participant_destroy(restored);

  return 0;
}

static char *test_participant_tick() {
  Settings *settings = xaynet_ffi_settings_new();
  with_keys(settings);
  with_url(settings);

  Participant *participant = xaynet_ffi_participant_new(settings);
  mu_assert("failed to create participant", participant != NULL);

  int status = xaynet_ffi_participant_tick(participant);
  mu_assert("missing no task flag", (status & PARTICIPANT_TASK_NONE));
  mu_assert("unexpected sum task flag", !(status & PARTICIPANT_TASK_SUM));
  mu_assert("unexpected update task flag", !(status & PARTICIPANT_TASK_UPDATE));
  mu_assert("unexpected set model flag",
            !(status & PARTICIPANT_SHOULD_SET_MODEL));
  mu_assert("unexpected made progress flag",
            !(status & PARTICIPANT_MADE_PROGRESS));
  // free memory
  xaynet_ffi_settings_destroy(settings);
  xaynet_ffi_participant_destroy(participant);

  return 0;
}

static char *all_tests() {
  mu_run_test(test_settings_new);
  mu_run_test(test_settings_set_keys);
  mu_run_test(test_settings_set_url);
  mu_run_test(test_settings);
  mu_run_test(test_participant_save_and_restore);
  mu_run_test(test_participant_tick);
  return 0;
}

int tests_run = 0;

int main(int argc, char **argv) {
  assert(xaynet_ffi_crypto_init() == OK);

  char *result = all_tests();
  if (result != 0) {
    fprintf(stderr, RED "ERROR: %s\n" RESET, result);
  } else {
    printf(GREEN "ALL TESTS PASSED\n" RESET);
  }
  printf("Tests run: %d\n", tests_run);

  return result != 0;
}
