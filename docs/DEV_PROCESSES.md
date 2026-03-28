# Development Process

## Clean Redeploy Process
1. rebuild the project with docker compose build (do not set a timeout because this takes a long time)
2. kill the conformance run and clean up any running containers that aren't part of this project cluster
3. remove any etcd data related to non project clusters
4. tear down the cluster with docker compose down
5. bring up the new cluster with docker compose up


## Fix Conformance Issues Process Loop
1. Write down all known issues into CONFORMANCE_FAILURES.md file
2. Continually fix any known issues, do not take shortcuts, do not ignore them even if they are architectural issues
3. As you fix issues implement a test that verifies your fix was correct and that the fix adheres to expected kubernetes runtime behavior
4. Make sure the component build works and tests pass
5. Make a git commit for this issue
6. Update the CONFORMANCE_FAILURES.md with the status of this fix
7. Do NOT arbitrarily go to the Clean Redeploy Process as this will delete the test results you need to reference for research fixing issues
8. Move on to the next known issue fix, using these process steps.
9. Do not stop this loop until all issues are fixed.

## Start Conformance Testing and monitor for errors
1. Start the conformance testing using the script
2. Monitor all container logs noting any errors.
3. Analyze any errors and track them in the CONFORMANCE_FAILURES.md
4. Start the Fix Conformance Issues Process Loop to start fixing while you monitor for more errors using the Fix Conformance Issues Process Loop
