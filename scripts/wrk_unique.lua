-- wrk script: add unique query param to bypass cache
local counter = 0

request = function()
    counter = counter + 1
    local path = wrk.path .. "?pagina=" .. counter
    return wrk.format("GET", path)
end
