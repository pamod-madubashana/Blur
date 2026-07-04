-- This M thing represents the the Lua module:
--  Everything in there will be passed to the plugin loader.
local M = {}

-- Every plugin should have an info table.
M.info = {
	name = "Hello", -- A very original name for our plugin
	version = 0.1 -- The version.
}

-- This will be called when the plugin loads
function M.init()
	print("Hello World!") -- Do the thing :D
	return true -- true means we loaded correctly
end

return M -- Return this plugin :D
