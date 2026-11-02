@RF-06 @RF-10
Feature: Resize the Albums panel and result columns
  Specification: specs/ui/resizable-widths.md
  Local preferences: specs/contracts/layout-widths.md

  @RF-10
  Scenario: Resize and reset the Albums panel
    Given the Albums panel is visible at its default width of 240 CSS pixels
    When I drag its right separator 80 CSS pixels to the right and release it
    Then the panel width is 320 CSS pixels
    And the table viewport becomes 80 CSS pixels narrower
    When I double-click that separator
    Then the panel returns to its default width

  @RF-06
  Scenario: Resize one column without squeezing its neighbors
    Given the table viewport is 1040 CSS pixels wide and no width override exists
    And the File and Folder columns are 150 and 200 CSS pixels wide
    When I drag the File column's right handle 80 CSS pixels to the right and release it
    Then File is 230 CSS pixels wide and Folder remains 200 CSS pixels wide
    And every other column retains its width
    And the table has horizontal scrolling when its tracks exceed the viewport
    When the viewport becomes 1390 CSS pixels wide
    Then File remains 230 CSS pixels wide and Folder becomes 400 CSS pixels wide
    When I focus the File resize handle and press Enter
    Then File and Folder return to responsive widths of 300 and 400 CSS pixels

  @RF-06
  Scenario Outline: Every column has an independent resize handle
    Given the table is visible with the "<column>" column in default mode
    When I reach its right-edge handle using horizontal scrolling if necessary
    And I resize it within the limits in the width specification
    Then its header and every data cell share the chosen width
    And its sort and the row selection are unchanged

    Examples:
      | column     |
      | state      |
      | file       |
      | folder     |
      | format     |
      | sampleRate |
      | bitDepth   |
      | duration   |
      | cutoff     |
      | bitrate    |
      | realBitrate |

  Scenario: Bounds and keyboard operation
    Given a vertical resize handle is focused
    When I press Right or Left
    Then its target changes width by 10 CSS pixels within its effective bounds
    And the handle announces its localized name and current width
    And no sort or row navigation occurs
    When I drag past its maximum and then move 10 CSS pixels left
    Then its width is 10 CSS pixels below its maximum

  Scenario Outline: Finishing an interrupted drag
    Given I have changed a width while dragging its handle
    When "<event>" occurs
    Then the last visible width is rounded, clamped and committed
    And later pointer movement does not resize anything

    Examples:
      | event          |
      | pointer-up     |
      | pointer-cancel |
      | window blur    |

  Scenario: Non-resize input retains its meaning
    Given a table with resize handles and a selected row
    When I click a header's sort button
    Then the existing sort action occurs
    When I click a resize handle without moving or use a secondary pointer button
    Then no width override, sort or selection change occurs

  @RF-13
  Scenario: Persist widths independently of the language
    Given I committed an Albums width of 320 and a File width of 230
    When I restart the application in the other supported language
    Then those widths are restored
    And the remaining columns are still in default mode
    And the resize handles have names in the active language
    When I reset File
    Then its override is removed without removing the Albums width preference

  Scenario: Unavailable or invalid local storage
    Given the saved width record is missing, invalid or unavailable as defined by the local preference contract
    When the main screen initializes
    Then the contract's defaults and per-field recovery rules apply
    When I resize a visible target and saving fails
    Then the chosen width remains usable in memory
    And no analysis error or status notice is introduced

  @RNF-04 @RNF-09
  Scenario: Analysis data and view changes preserve widths
    Given overridden widths and a selected track with a Unicode filename
    When a new run reports valid tracks and a corrupt track
    And I sort and filter the results
    Then the width preferences remain unchanged
    And names, verdicts, errors, selection rules and CSV behavior follow their existing specifications

  Scenario: Empty and modal states
    Given the screen has no sidebar
    Then there is no sidebar resize handle
    Given a visible empty or no-matches result table
    Then its column headers remain resizable
    When Settings is open
    Then the background resize handles are inert

  @RF-13
  Scenario: Rendered table layout remains usable after resizing
    Given the M1 rendered matrix in both languages and both supported test window sizes
    When I resize the sidebar and every column through their default, minimum and maximum widths
    Then headers and cells stay aligned and no header text overlaps a data row
    And every column's handle remains reachable
    And sorting, the Real bitrate tooltip, vertical scrolling and End/Home selection remain usable
