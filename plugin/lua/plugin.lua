local str_find = string.find
local str_sub = string.sub
local str_gmatch = string.gmatch
---@param uri string # The uri of file
---@param text string # The content of file
---@return { start: integer, finish: integer, text: string }[] | string | nil
function OnSetText(uri, text)
    -- ignore .vscode dir, extension files (i.e. natives), and other meta files
    if str_find(uri, '[\\/]%.vscode[\\/]') or str_sub(text, 1, 8) == '---@meta' then return end
    -- ignore files using fx asset protection
    if str_sub(text, 1, 4) == 'FXAP' then return '' end
    local diffs = {}
    local count = 0
    -- prevent diagnostic errors in fxmanifest.lua and __resource.lua files
    if str_find(uri, 'fxmanifest%.lua$') or str_find(uri, '__resource%.lua$') then
        count = count + 1
        diffs[count] = {
            start = 1,
            finish = 0,
            text = '---@diagnostic disable: undefined-global\n'
        }
    end
    -- prevent diagnostic errors from safe navigation (foo?.bar and foo?[bar])
    for safeNav in str_gmatch(text, '()%?[%.%[]+') do
        count = count + 1
        diffs[count] = {
            start  = safeNav,
            finish = safeNav,
            text   = '',
        }
    end
    -- prevent "need-check-nil" diagnostic when using safe navigation
    -- only works for the first index, and requires dot notation (i.e. mytable.index, not mytable["index"])
    for pre, whitespace, tableStart, tableName, tableEnd in str_gmatch(text, '([=,;%s])([%s]*)()([_%w]+)()%?[%.%[]+') do
        count = count + 1
        diffs[count] = {
            start  = tableStart - 1,
            finish = tableEnd - 1,
            text   = ('%s(%s or {})'):format(whitespace == '' and pre or '', tableName)
        }
    end

    -- handle Lua 5.4 shorthand object initialization (e.g. { .name, .age })
    -- Pattern to match standalone shorthand notation in tables
    -- This pattern looks for dots followed by identifiers that are not part of other expressions

    -- First pass: Parse the entire text to track state (inside comment, string, table, etc.)
    local i                 = 1
    local len               = #text
    local in_string_single  = false     -- Inside 'string'
    local in_string_double  = false     -- Inside "string"
    local in_long_string    = false     -- Inside [[long string]]
    local in_comment        = false     -- Inside --comment
    local in_long_comment   = false     -- Inside --[[comment]]
    local table_depth       = 0         -- Track table nesting level
    local table_regions     = {}        -- Store start/end positions of tables with their depth

    while i <= len do
        local c = text:sub(i, i)
        local next_c = i < len and text:sub(i + 1, i + 1) or ""

        -- Handle strings
        if not in_comment and not in_long_comment then
            if c == "'" and not in_string_double and not in_long_string then
                in_string_single = not in_string_single
            elseif c == '"' and not in_string_single and not in_long_string then
                in_string_double = not in_string_double
            elseif c == "[" and next_c == "[" and not in_string_single and not in_string_double then
                in_long_string = true
                i = i + 1 -- Skip the next [
            elseif c == "]" and next_c == "]" and in_long_string then
                in_long_string = false
                i = i + 1 -- Skip the next ]
            end
        end

        -- Handle comments
        if not in_string_single and not in_string_double and not in_long_string then
            if c == "-" and next_c == "-" and not in_comment and not in_long_comment then
                in_comment = true
                i = i + 1 -- Skip the next -

                -- Check for long comment
                if i + 1 <= len and text:sub(i + 1, i + 1) == "[" and text:sub(i + 2, i + 2) == "[" then
                    in_long_comment = true
                    in_comment = false
                    i = i + 2 -- Skip the [[
                end
            elseif c == "\n" and in_comment then
                in_comment = false
            elseif c == "]" and next_c == "]" and in_long_comment then
                in_long_comment = false
                i = i + 1 -- Skip the next ]
            end
        end

        -- Track table regions (only outside of strings and comments)
        if not in_string_single and not in_string_double and not in_long_string and not in_comment and not in_long_comment then
            if c == "{" then
                table_depth = table_depth + 1
                table.insert(table_regions, { start = i, depth = table_depth })
            elseif c == "}" and table_depth > 0 then
                -- Find matching open table
                for j = #table_regions, 1, -1 do
                    if table_regions[j].depth == table_depth and not table_regions[j].finish then
                        table_regions[j].finish = i
                        break
                    end
                end
                table_depth = table_depth - 1
            end
        end

        i = i + 1
    end

    -- Second pass: Find shorthand initializations in valid table contexts
    for _, region in ipairs(table_regions) do
        if region.finish then -- Only process complete table regions
            local table_content = text:sub(region.start, region.finish)

            -- Parse the table content character by character to find .identifier patterns
            local j = 1
            while j <= #table_content do
                -- Find .identifier pattern
                if table_content:sub(j, j) == "." then
                    -- Check if this is a standalone .identifier (not part of table.field or string)
                    local prev_char = j > 1 and table_content:sub(j - 1, j - 1) or ""

                    -- Valid prev chars are whitespace, comma, or {
                    local valid_prev = prev_char:match("[%s,{]") or j == 1

                    if valid_prev then
                        -- Extract the identifier
                        local identifier_match = table_content:sub(j + 1):match("^([_%w]+)")

                        if identifier_match then
                            local next_char_pos = j + 1 + #identifier_match
                            local next_char = next_char_pos <= #table_content and
                            table_content:sub(next_char_pos, next_char_pos) or ""

                            -- Valid next chars are whitespace, comma, or }
                            local valid_next = next_char:match("[%s,}]") or next_char_pos > #table_content

                            if valid_next then
                                -- We found a valid shorthand initialization
                                count = count + 1
                                diffs[count] = {
                                    start = region.start + j - 1,
                                    finish = region.start + j + #identifier_match - 1,
                                    text = identifier_match .. " = true"
                                }
                            end
                        end
                    end
                end
                j = j + 1
            end
        end
    end

    return diffs
end
