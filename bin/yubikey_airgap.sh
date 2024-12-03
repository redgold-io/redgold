# First reset everything
ykman config reset
ykman piv reset
ykman oath reset
ykman openpgp reset

# PIV (using first 24 bytes of entropy)
PIV_KEY=${MASTER_ENTROPY:0:48}  # take first 48 hex chars (24 bytes)
ykman piv change-management-key --management-key $PIV_KEY

# OATH (using next 20 bytes for a secret)
OATH_KEY=${MASTER_ENTROPY:12:40}  # next 40 hex chars (20 bytes)
# For each OATH credential you want to add:
ykman oath accounts add -t "account_name" $OATH_KEY

# OpenPGP (can use full 32 bytes)
# First generate your GPG key using your entropy
# Then import to GPG card:
gpg --edit-card
# In GPG shell:
admin
key-import

# For U2F/FIDO - this actually can't be customized with own entropy
# The YubiKey generates these credentials internally

# Optional: protect PIV operations with PIN
ykman piv access change-pin  # default is 123456
ykman piv access change-puk  # default is 12345678