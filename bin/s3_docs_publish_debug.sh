export S3_DOCS_PREFIX="s3://redgold-docs"

export BRANCH=${1:-dev}
export DEST="$S3_DOCS_PREFIX-$BRANCH"
echo "Publishing to $DEST"
aws s3 rm --recursive $DEST && aws s3 cp --recursive ./docs/dist/ $DEST/
