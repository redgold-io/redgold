Overall paranoid guide to security assuming no knowledge at all, 
skip sections as needed if you already know what you're doing. 


Use Firefox or another privacy related browser. Install all the 
standard [privacy extensions](https://addons.mozilla.org/en-US/firefox/) for additional help. 
Setup a password manager account with [Lockwise](https://www.mozilla.org/en-US/firefox/lockwise/) 
or some other supported password manager. Use a strong 'hot password'. This is your main 
password for generating all subsequent managed passwords. Password manager should be 
setup with an email and 2FA and recovery codes (covered next). Do not use this password 
anywhere else besides the password manager. This is the first password and is the least 
secure as it requires interactions with external 'hot' services. 

For email, use a secure service like [ProtonMail](protonmail.com) -- this will also 
require you to setup 2FA and recovery codes. It's sufficient to re-use the 1st master 
hot password here as your email and password manager should be considered roughly the 
same level of security given how many services allow resets. This also allows recovery 
despite losing all password manager details -- which is important.

For 2FA use a service like [Authy](https://authy.com/) which is compatible with previous 
steps. Avoid SMS. If you're super clever use a yubikey

Install a client-side encryption tool like [Cryptomator](https://cryptomator.org/), this is 
better than tools like Veracrypt which produce an entire encrypted volume. Create a new  
password (the 2nd password in total,) which is not the same as the previous password. 
Do not ever use this password anywhere or submit it to any external computer. This 
step relies on the security of the system you're running on, so it's a partially hot 
password (we'll call it semi-hot) since it's assumed you'll most likely run this on a regular computer that has 
internet access for synchronization and backup. This can produce a recovery key if desired. 
Print that recovery key out and store somewhere secure if needed, otherwise not 
required so long as password properly memorized.

Install backup and sync services. [pCloud](pcloud.com/) is most convenient for synchronization 
since it allows you to select an arbitrary folder instead of forcing a specific one. 
For other services, nest the folders together. I.e. 
`~/Google Drive/Sync/Dropbox/..etc/your_vault` -- at least 2 services should be used 
in case of failure. 

Next you'll need to buy a cheap linux-compatible laptop which acts as the cold computer. 
[Example cheap laptop](https://www.amazon.com/gp/product/B081V6W99V/ref=ppx_yo_dt_b_search_asin_title?ie=UTF8&psc=1). 
Also purchase a [USB drive](https://www.amazon.com/gp/product/B08GYM5F8G/ref=ppx_yo_dt_b_search_asin_title?ie=UTF8&psc=1) for booting a live OS from, 
and a secondary [USB drive](https://www.amazon.com/gp/product/B07D7PDLXC/ref=ppx_yo_dt_b_search_asin_title?ie=UTF8&psc=1) for copying over data.
Install [TAILS](https://tails.boum.org/) or another equivalent secure live OS onto the USB drive. Never ever 
connect this computer to the internet, not even for updates or anything else. Keep it 
secure away from any usage other than dedicated password operations. Boot the laptop 
into it's BIOS -- hold a key dependent on manufacturer, 
[Hold F2 for above linked example laptop](https://www.asus.com/us/support/FAQ/1008829/) -- and select the USB drive
to boot into. Copy the CLI binary over from the main hot computer onto the secondary 
USB drive and open it on the cold computer. Run the CLI in a terminal and input 
your cold password (this is the 3rd password and should NOT be the same as the other two) 
to generate mnemonics. The CLI should output metadata allowing you to 
store information about what offsets were used during mnemonic generation (so you 
can generate many mnemonics from the same starting password and keep track of which have 
been used so far.) Copy this metadata back to the secondary USB drive, and load it onto the 
main computer. Store it in the metadata database for later usage, and keep that database 
inside the cryptomator vault.

Set the hashing rounds as high as is reasonable to generate in enough time. Several 
million rounds should be relatively fast. Avoid using small numbers of rounds.

When generating the cold password -- make sure you use sufficient entropy. Short passwords 
at this step are dangerous, as they won't gather enough entropy to generate a 
sufficiently random mnemonic. Mnemonics are typically generated using a randomized seed 
collected with hardware entropy. As long as you use a sufficiently matching amount of 
entropy it should be secure. This requires memorizing a very long password. 100+ characters 
is ideal, but this is much easier than memorizing a random mnemonic because it can 
be built with whatever associations are easiest for you to memorize. A mixture 
of dictionary words, symbols, fragments of other passwords you're familiar with, and more 
is ideal. Try easy association expansions. If part of your password for instance is 'dog', 
this can be expanded to 'dogcat' to lengthen it and is an easy to memorize association.

So long as you have some symbols and other standard password features, most of the length 
in generation should be focused on just expanding it for entropy purposes. 

Important note -- do not ever copy any mnemonics intended to be used with hardware wallets 
onto the USB drive. Manually enter them into a Trezor or Ledger wallet from the terminal 
output. Do not write any data to disk on the cold computer. 

Another important note: never re-use the same password + offset + hashing function type with a different round 
combination. Any given input will hash rounds sequentially, making all later rounds 
derivative of earlier rounds. If an earlier round mnemonic is compromised, then all 
later rounds are compromised as well.

If generating mnemonics for use with external servers, copy them to the main computer 
and deploy them over secure SSH. Trezor can be used for generating SSH keys. 

This process will yield an unlimited number of mnemonics. But that's not enough. 

To secure this even further, you should also rely on purely randomly generated mnemonics with 
semi-hot passphrases used. Use either a cold computer or a Ledger/Trezor to generate a random mnemonic. 
This is a guarantee against compromise of the potential entropy issues with any brain wallet from the first step, 
in the event you didn't use a sufficient supply of entropy or rounds. 

There's a choice here in securing it, if using a physical copy alone, it should have multiple redundant 
backups in the event of natural disaster / theft. More realistically, to gain the benefits of 
pure randomness you need only back it up on the earlier mentioned Cryptomator / client side encrypted cloud volume.

Using the same semi-hot password as before should be fine, as any keylogger which can break the cryptomator 
volume will also pick up the passphrase when entered in a Trezor/Ledger UI -- so a unique secondary 
semi-hot password does not offer many benefits. 

The next layer of protection should be the same process applied and stored to your friends. Brain wallets here 
have little use as they are redundant with the first solution and don't require a primary backup. Instead 
you gain security more from a purely random key with a passphrase. Give each person a primary mnemonic, re-use 
your same semi-hot passphrase for each. This allows you to rotate keys independently with each person 
as opposed to all at once like Shamir. 

The final step is Shamir shares, again which should be protecting purely randomly generated mnemonic 
with the same semi-hot passphrase as the final security. Generate one Shamir group with a number of 
shares according to friends, and distribute. This is much less flexible as any of the previous schemes, 
as it requires the entire set of keys to be rotated at once should you wish to change anything. 

All of these keys can be used in multi-signature schemes and thresholds set independently of the key 
distribution. As long as these keys are maintained transactions can update signature thresholds 
without having to rotate any keys. 

The final note on recovery procedures is to generate a large amount of metadata about yourself as a 
recovery step. In the event of failures of everything else, you should build a metadata prompt 
using personal information that is easier to recover than passwords. I.e. name, date of birth, town 
of birth, drivers license number, passport number, secret questions / answers, email, etc.

These metadata items can be either concatted or padded with randomness to generate a set of metadata keys, 
allowed to be used in threshold schemes for recovery.

Metadata can also be used to pad primary cold passwords, but should only be used with high degree of confidence 
of recall (i.e. maybe just name/dob in case others forgotten.)

TODO: 
Generation of one time pads + encryption of them.
PGP key management for above ^ + email.