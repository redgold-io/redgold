# Release Process

To form a secure release process, it is necessary to collect signatures from peers who have reviewed the code and 
verified the commit hashes match the appropriate built executable and built docker images. This means that the 
fundamental structure for verifying the correct software version should be signed and updated by each peer's own 
metadata, requiring a signature to guarantee the new version. 

Not all peers can necessarily sign this information nor participate in this process due to technical capacity, but 
scores associated with all reviewers should be used to determine majorities associated with choice of the current 
software hash. As it is undesirable to have to manually upgrade versions all the time for many reasons, an 
auto-update process based on this peer metadata should replace the functionality typically employed by a CI/CD 
deployment process. For peers who wish to manually manage this process, the auto-update should be easily disabled 
so that their own CI can be used in the management of version changes -- however it is highly recommended that peer 
information still be used to determine when they have diverged from the current majority software fork in order to 
ensure continuing participation in the majority network.
