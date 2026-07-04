print_api("amax/init.lua {")

local loader = require("amax.loader")

if loader.ok then
	local n = loader.n_loaded
	local e = loader.n_failed
	local t = n + e

	if t == 0 then  -- no plugins
		print_api(" - No plugins loaded. Was that intentional?")
	elseif n == t then -- all good
		print_api(" - Loaded all " .. tostring(n) .. " plugins.")
	elseif e == t then -- all bad
		print_api(" - Failed to load all " .. tostring(e) .. " plugins.")
	elseif 0 < e then -- some bad
		print_api(" - Failed to load " .. tostring(e) .. " / " .. tostring(t) .. " plugins.")
	else            -- unreachable but I'll keep it here :>
		print_api(" - Out of " .. tostring(t) .. " plugins, " .. tostring(n) .. " loaded OK, and " .. tostring(e) .. "failed to load")
	end
else
	print_api(
	" - [ERROR]: amax.loader.ok == false; Didn't load any plugins - The problem is probably in amax/plugins.lua")
end

BlurAPI.loaded_plugins = loader.loaded_plugins


print_api("}")
