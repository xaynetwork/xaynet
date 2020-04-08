#!/usr/bin/env bash

WORKDIR="$(git rev-parse --show-toplevel)"
# Save the git HEAD before running the script
HEAD=
# Latest tag
PREV_TAG=
# Commit that corresponds to the latest tag
PREV_TAGGED_COMMIT=

# Latest version numbers
PREV_MAJOR=
PREV_MINOR=
PREV_PATCH=

# New version numbers
MAJOR=
MINOR=
PATCH=

# Return the new version number
version() {
    echo "${MAJOR}.${MINOR}.${PATCH}"
}

# Return the previous version number
prev_version() {
    echo "${PREV_MAJOR}.${PREV_MINOR}.${PREV_PATCH}"
}


# Find and parse the latest tag, and populate the global variables
fetch_latest_version() {
    local tag_regex='^v[0-9]\.[0-9]\.[0-9]$'

    PREV_TAG=$(git describe --tags --abbrev=0)
    PREV_TAGGED_COMMIT=$(git rev-list -n 1 "${PREV_TAG}")
    echo "latest tag found: ${PREV_TAG} (commit ${PREV_TAGGED_COMMIT})"

    if ! [[ ${PREV_TAG} =~ ${tag_regex} ]] ; then
        echo "error: invalid tag ${PREV_TAG}" >&2
        exit 1
    fi

    PREV_MAJOR=${PREV_TAG:1:1}
    PREV_MINOR=${PREV_TAG:3:1}
    PREV_PATCH=${PREV_TAG:5:1}

    MAJOR=${PREV_MAJOR}
    MINOR=${PREV_MINOR}
    PATCH=${PREV_PATCH}
}

# Check that the working directory doesn't have un-committed changes. If it
# does, error out.
check_workdir_is_clean() {
    if [ -z "$(git status --untracked-files=no --porcelain)" ]; then
        echo "git working directory is clean, continuing"
    else
        echo "git working directory is dirty, aborting" 2>&1
        exit 1
    fi
}

# A helper function for interactively asking the users whether the script
# should continue or not
ask_yes_or_no() {
    select yn in "Yes" "No"; do
        case $yn in
            Yes )
                echo "continuing"
                break
                ;;
            No )
                echo "aborting" 2>&1
                exit 1
                ;;
        esac
    done
}

# Print a message explaining what the script does, and how to undo the changes
# if necessary
disclaimer() {
    cat << EOF
***********************************
        IMPORTANT
***********************************

This script modifies the git commit history. If anything goes wrong, or if you
have a doubt, you can always rollback to where this script start by running:

    git reset --hard ${HEAD}

This script will:

1. Find the latest tag on the current branch
2. Make sure that the CHANGELOG.md file was updated since this tag was pushed
3. Update the version number in various files in the repository, and commit
   these changes
4. Create a new annotated tag

EOF
}

# Print a help message
usage() {
    cat << EOF
./bump_version.sh [-h|--help] [-M|--major] [-m|--minor] [-p|--patch]

bump_version.sh is used for bumping the previous version number and creating a
new tag.

OPTIONS:

    -h|--help:
        print this help message

    -M|--major:
        bump the major version number

    -m|--minor:
        bump the minor version number

    -p|--patch:
        bump the patch version number
EOF
}

# Make sure the CHANGELOG was updated, and ask the user to double check the
# changes
check_changelog_was_updated() {
    diff() {
        git --no-pager diff "${PREV_TAGGED_COMMIT}" HEAD CHANGELOG.md
    }

    if [ "$(diff | wc -l)" -eq 0 ] ; then
        echo "error: the CHANGELOG has not been updated since ${PREV_TAG}" 2>&1
    fi

    echo "The CHANGELOG has been updated since ${PREV_TAG}"
    diff
    echo "Does the change above look correct for v$(version)"
    ask_yes_or_no
}

# Small helper to update the version number in a file, using sed
set_version_in_file() {
    local sed_expr=${1}
    local file=${2}

    echo "Setting version to $(version) in ${file}"
    sed -i "${sed_expr}" "${file}"
}

# Update the version numbers in various files, and ask confirmation from the
# user before committing these changes.
update_versions() {
    local py_files=

    set_version_in_file 's/^version = ".*"$/version = "'"$(version)"'"/g' rust/Cargo.toml

    py_files=(
        python/sdk/xain_sdk/__version__.py
        python/aggregators/xain_aggregators/__version__.py
    )
    for f in "${py_files[@]}" ; do
        set_version_in_file 's/[0-9]\.[0-9]\.[0-9]/'"$(version)"'/g' ${f}
    done

    for f in swagger/*.yml ; do
        set_version_in_file 's/\(\s\+version: \)[0-9]\.[0-9]\.[0-9]/\1'"$(version)"'/g' "${f}"
    done


    if [ "$(git --no-pager diff | wc -l)" -eq 0 ] ; then
        echo "No changes were made, it seems that the version files were already updated to $(version)"
        echo "Do you want to continue?"
        ask_yes_or_no
    else
        git --no-pager diff
        echo "Do you want to commit the changes above?"
        ask_yes_or_no
        git add rust/Cargo.toml python/sdk/xain_sdk/__version__.py python/aggregators/xain_aggregators/__version__.py swagger/*.yml
        git commit -m "bump version $(prev_version) -> $(version)"
    fi
}

build_python_package() {
    echo "Building python package at $(pwd)"
    python setup.py sdist bdist_wheel
    echo "Python package at $(pwd) built successfully"
}

cargo_publish_dry_run() {
    echo "Checking that the Rust package is ready to be published"
    cargo publish --dry-run
    echo "The Rust package is ready to be published"
}

main() {
    local bump_major=false
    local bump_minor=false
    local bump_patch=false

    if [ "$#" -eq 0 ]; then
        usage
        exit 0
    fi

    cd "${WORKDIR}"
    check_workdir_is_clean

    while (( "$#" )); do
        case "$1" in
            -M|--major)
                bump_major=true
                shift
                ;;
            -m|--minor)
                bump_minor=true
                shift
                ;;
            -p|--patch)
                bump_patch=true
                shift
                ;;
            -h|--help|help)
                usage
                exit 0
                ;;
            *)
                echo "error: unsupported argument \"$1\"" 2>&1
                usage
                exit 1
                ;;
        esac
    done


    HEAD=$(git rev-parse HEAD)
    disclaimer
    fetch_latest_version

    if [ "$bump_major" = true ] ; then
        MAJOR=$((PREV_MAJOR + 1))
    fi

    if [ "$bump_minor" = true ] ; then
        MINOR=$((PREV_MINOR + 1))
    fi

    if [ "$bump_patch" = true ] ; then
        PATCH=$((PREV_PATCH + 1))
    fi

    if [ "$(prev_version)" = "$(version)" ] ; then
        echo "error: new version is the same than previous version" 2>&1
        exit 1
    fi

    echo "Bumping version from $(prev_version) to $(version)"
    ask_yes_or_no

    update_versions
    check_changelog_was_updated

    (cd python/sdk && build_python_package)
    (cd python/aggregators && build_python_package)
    (cd rust && cargo_publish_dry_run)

    echo "Tagging ${HEAD} as \"v$(version)\""
    git tag -a "v$(version)" -m "release v$(version)"

    echo "Done!"

    cat << EOF
You can now publish the Python packages:

    (cd python/sdk && twine upload dist/*)
    (cd python/aggregators && twine upload dist/*)

And publish the Rust package:

    (cd rust && cargo publish)

Finally: push the new tag to Github:

    git push <remote> master --tags

EOF
}

set -e
main "$@"
