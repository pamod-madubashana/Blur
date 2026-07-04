-- This M thing represents the Lua module:
--  Everything in there will be passed to the plugin loader.
local M = {
	fps = 0,
}


local CONFIG = {
	fps_menu = 120, -- set to 0 for unlimited
	fps_race = 60,
}

local RACE_SCREENS = {
	["Ingame\\IntroRace.lua"] = true,
	["Ingame\\Hud.lua"] = true,
	["Ingame\\HUD.lua"] = true,
	["Ingame\\SpWrecked.lua"] = true,
}

-- Every plugin should have an info table.
M.info = {
	name = "FPS",
	description = "Changes FPS values depending on if you're racing or not.",
	version = 0.1,
}

function M.GoScreen(screen, _opts)
	local update_fps = M.fps
	if RACE_SCREENS[screen] then
		update_fps = CONFIG.fps_race
	else
		update_fps = CONFIG.fps_menu
	end

	if not (M.fps == update_fps) then
		M.fps = update_fps
	end
	BlurAPI.set_fps(update_fps)
end


-- This will be called when the plugin loads
function M.init()
	M.fps = CONFIG.fps_menu
	BlurAPI.set_fps(M.fps)
	return true -- true means we loaded correctly
end

return M -- Return this plugin
