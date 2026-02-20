-- log_errors.lua

-- Open the log file in append mode
-- Note: If running in Docker, ensure this path is mapped to your host
local file = io.open("wrk_debug2.log", "a")

-- 1. Setup Header
file:write("\n========================================\n")
file:write("NEW BENCHMARK RUN: " .. os.date("%Y-%m-%d %X") .. "\n")
file:write("========================================\n")
file:flush()

-- 2. Response Logic
-- This triggers ONLY if the server sends a valid HTTP status line
response = function(status, headers, body)
   -- Log if it's an error (not 200) or if the body is unexpectedly empty
   if status ~= 200 or (body == nil or #body == 0) then
      file:write(string.format("[%s] ERROR - Status: %s\n", os.date("%X"), status))

      file:write("HEADERS:\n")
      for k, v in pairs(headers) do
         file:write(string.format("  %s: %s\n", k, v))
      end

      -- If there is a body (like a panic message), log the first 200 chars
      if body and #body > 0 then
         file:write("BODY SAMPLE: " .. body:sub(1, 200) .. "\n")
      else
         file:write("BODY: (empty)\n")
      end

      file:write("------------------------------\n")
      file:flush()
   end
end

-- 3. Final Summary Logic
-- This triggers even if the server crashes, providing the socket-level data
done = function(summary, latency, requests)
   file:write("\n--- BENCHMARK COMPLETE ---\n")
   file:write(string.format("Total Requests: %d\n", summary.requests))
   file:write(string.format("Total Duration: %.2f sec\n", summary.duration / 1000000))
   file:write("\nSOCKET-LEVEL ERRORS:\n")
   file:write(string.format("  Connect: %d\n", summary.errors.connect))
   file:write(string.format("  Read:    %d\n", summary.errors.read))
   file:write(string.format("  Write:   %d\n", summary.errors.write))
   file:write(string.format("  Timeout: %d\n", summary.errors.timeout))
   file:write("========================================\n\n")

   file:close()

   -- Also print a quick alert to your terminal
   print("\n[LUA] Test finished. Check wrk_debug.log for details.")
   if summary.errors.read > 0 then
      print(string.format("[LUA] ALERT: %d read errors detected at the socket level!", summary.errors.read))
   end
end