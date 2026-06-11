-- errors.lua
local file = io.open("wrk_debug.log", "a")
local error_count = 0
local max_errors_to_log = 100 -- Prevents disk bloat

-- Header for a fresh run
file:write(string.format("\n\n%s\n", string.rep("=", 60)))
file:write(string.format("NEW BENCHMARK RUN: %s\n", os.date("%X")))
file:write(string.format("%s\n", string.rep("=", 60)))
file:flush()

-- Setup: Runs once per thread
setup = function(thread)
   thread:set("id", error_count)
end

-- Response: Triggered ONLY when the server sends a valid HTTP status
response = function(status, headers, body)
   if status ~= 200 then
      error_count = error_count + 1

      if error_count <= max_errors_to_log then
         file:write(string.format("[%s] HTTP ERROR %d\n", os.date("%X"), status))

         -- Catching the "Content-Length" vs "Actual Body" mismatch
         local body_len = body and #body or 0
         file:write(string.format("Body Length Received: %d\n", body_len))

         if body_len > 0 then
            file:write("Raw Body: " .. body:sub(1, 100) .. "\n")
         end

         file:write("------------------------------\n")
         file:flush()
      end
   end
end

-- Done: The most important part for your "Read Errors"
done = function(summary, latency, requests)
   file:write("\n--- FINAL EXECUTION SUMMARY ---\n")
   file:write(string.format("Total Requests: %d\n", summary.requests))
   file:write(string.format("Total Duration: %.2f seconds\n", summary.duration / 1000000))

   file:write("\nSOCKET ERRORS (The 'Why'):\n")
   file:write(string.format("  Connect Errors: %d\n", summary.errors.connect))
   file:write(string.format("  Read Errors:    %d (Server dropped the ball)\n", summary.errors.read))
   file:write(string.format("  Write Errors:   %d\n", summary.errors.write))
   file:write(string.format("  Timeout Errors: %d\n", summary.errors.timeout))

   if summary.errors.read > 0 then
      file:write("\nDIAGNOSIS:\n")
      file:write("High Read Errors + No HTTP Status in log = TCP RST/Abort.\n")
      file:write("Check if your Database pool is starving the worker threads.\n")
   end

   file:write(string.format("\n%s\n", string.rep("=", 60)))
   file:close()

   print("\n[DEBUG] Results written to wrk_debug.log")
end