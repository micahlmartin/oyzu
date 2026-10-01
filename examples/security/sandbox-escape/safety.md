# Running the negative probes

Do not run these probes against real home-directory files or external services. The future sandbox harness must create a throwaway sentinel and loopback listener, then invoke the probes inside the build executor. Native execution would intentionally be able to read the sentinel and is not a passing security test. No escape probe is executed by the current native smoke runner.
