# Configuration

Currently configuration happens two fold.

## `config.cfg`

On the initial run of the library the `xain_fl.config.init_config()` function
will be executed and initialize a `config.cfg` file in the projects root directory.
If the user wants to run benchmarks and wants results to be uploaded to S3 he will need to replace the
`ACCESSIBLE_S3_BUCKET_FOR_RESULTS_TO_BE_UPLOADED` string in the `config.cfg` with an actual S3 bucket name. The bucket
should be accessible with the currently active AWS credentials (either default credentials or the currently set AWS_PROFILE).

## abseil.io flags

The second form of configuration happens via abseil.io flags. All entry points are configured as abseil.io apps which
accept various flags. To find out what valid flag exist one can run any entry point with the `--helpfull` flag.
For example

```shell
$ aggregate --helpfull
```
