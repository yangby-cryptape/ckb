#!/bin/bash
set -ev

cargo sweep -s

# Run test only in master branch and pull requests
RUN_TEST=false
# Run integration only in master, develop and rc branches
RUN_INTEGRATION=false
if [ "$TRAVIS_PULL_REQUEST" != false ]; then
  echo "\$TRAVIS_PULL_REQUEST_BRANCH = $TRAVIS_PULL_REQUEST_BRANCH"
  LAST_COMMIT_MSG="$(git log --max-count 1 --format="%B" $TRAVIS_PULL_REQUEST_BRANCH)"
  echo "Last commit message is \"${LAST_COMMIT_MSG}\""
  if [[ "${LAST_COMMIT_MSG}" =~ ^[a-z]+:\ \[skip\ tests\]\  ]]; then
      :
  elif [[ "${LAST_COMMIT_MSG}" =~ ^[a-z]+:\ \[only\ integration\]\  ]]; then
    RUN_INTEGRATION=true
  elif [[ "${LAST_COMMIT_MSG}" =~ ^[a-z]+:\ \[all\ tests\]\  ]]; then
    RUN_TEST=true
    RUN_INTEGRATION=true
  else
    RUN_TEST=true
  fi
else
  RUN_INTEGRATION=true
  if [ "$TRAVIS_BRANCH" = master ]; then
    RUN_TEST=true
  fi
fi

echo "\${RUN_TEST} = ${RUN_TEST}"
echo "\${RUN_INTEGRATION} = ${RUN_INTEGRATION}"
