#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

PRE_PUSH_HOOK="./.git/hooks/pre-push"

cd $DIR/../

if [ -f "$PRE_PUSH_HOOK" ]; then
    echo "$PRE_PUSH_HOOK exist"
else 
    echo "#!/bin/sh" >> $PRE_PUSH_HOOK
    echo "./scripts/test.sh" >> $PRE_PUSH_HOOK
    
    chmod +x $PRE_PUSH_HOOK

    echo "$PRE_PUSH_HOOK created"
fi
