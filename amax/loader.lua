local M = {
	loaded_plugins = {},
	n_failed = 0,
	n_loaded = 0,
	ok = false,
}

package.path =
	".\\amax\\plugins\\?.lua;"
	..
	".\\amax\\plugins\\?.luac;"
	.. package.path
package.cpath =
	".\\amax\\plugins\\?.dll;"
	.. package.cpath


---Loads plugins from plugins/plugins.lua
local plugins_ok, plugins_rs = pcall(require, "amax.plugins.plugins")
if not plugins_ok then
	-- Consider just crashing the game here?
	print_api(" - [ERROR] " .. plugins_rs)
	return M
end

for k, plugin in pairs(plugins_rs) do
	if not plugin.info then
		print_api(" - ...ERROR: plugins[" .. k .. "] - no plugin.info {} table.")
		M.n_failed = M.n_failed + 1;
	elseif not plugin.init then
		print_api(" - ...ERROR: plugins[" .. k .. "] - no plugin.init() function.")
		M.n_failed = M.n_failed + 1;
	else
		print_api(" >> LOADING: " .. plugin.info.name .. "@" .. plugin.info.version)
		local ok, rs = pcall(plugin.init)
		if not ok then
			print_api("\t...ERROR: " .. rs)
			M.n_failed = M.n_failed + 1;
		elseif not rs then
			print_api("\t...NOT LOADED: init() returned false")
			M.n_failed = M.n_failed + 1;
		else
			M.loaded_plugins[#M.loaded_plugins + 1] = plugin
			print_api("\t...OK!")
			M.n_loaded = M.n_loaded + 1;
		end
	end
	print_api()
end

M.ok = true
return M
