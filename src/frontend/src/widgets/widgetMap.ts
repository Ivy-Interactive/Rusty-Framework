// Eager vs lazy: how to choose when adding a widget.
//
// Default to lazy - `lazyWithRetry(() => import("@/widgets/<dir>/<Widget>"))`, naming the concrete
// module and never a barrel. The renderer already wraps any lazy component in <Suspense>
// (widgetRenderer.tsx), so lazy needs no other change. Keep an entry eager only for one of the
// reasons below; if none applies, it is lazy.
//
// Stay eager when:
//   - It is the loading fallback itself ($loading / LoadingScreen). It cannot be behind Suspense.
//   - Measurement shows lazifying it makes the initial download BIGGER. Ivy.AppHost is the known
//     case: it hosts a whole app and carries the signalr transport, and lazifying it cost +11.4 kB
//     and fragmented 31 extra chunks.
//
// Two things that are NOT reasons, both measured:
//   - "It is small." Eager cost is not the widget's own bytes, it is what its dependencies drag in,
//     and a shared dependency stays eager until its LAST eager importer leaves. Ivy.Icon lazified
//     alone moves 0 bytes (25 modules reach @/components/Icon); the whole eager list lazified moves
//     666.6 kB (-25 %) and drops the 475 kB Icon chunk out of the eager graph.
//   - "It is needed on first paint." Widgets are chosen by server-driven type strings at runtime, so
//     no entry here is known to be on the first screen. Suspense covers the fetch.
//
// A barrel that re-exports a lazily-imported widget defeats the split even when the import() names
// the concrete module; the barrel must not re-export it. The assert-lazy-chunks plugin in
// vite.config.mjs fails the build on exactly that mistake, so `pnpm run build` catches it - but it
// does NOT measure eager cost, so a green build does not mean an entry belongs on the eager side.
// For that, walk static import edges from the entry chunk - see README.md, "Module Graph and Lazy
// Loading".

import { LoadingScreen } from "@/components/LoadingScreen";
import { AppHostWidget } from "@/widgets/primitives/AppHostWidget";
import { lazyWithRetry } from "@/lib/lazyWithRetry";

export const widgetMap = {
  $loading: LoadingScreen,

  // Primitives
  "Ivy.TextBlock": lazyWithRetry(() =>
    import("@/widgets/primitives/TextBlockWidget").then((m) => ({
      default: m.TextBlockWidget,
    })),
  ),
  "Ivy.RichTextBlock": lazyWithRetry(() =>
    import("@/widgets/primitives/RichTextBlockWidget").then((m) => ({
      default: m.RichTextBlockWidget,
    })),
  ),
  "Ivy.Markdown": lazyWithRetry(() => import("@/widgets/primitives/MarkdownWidget")),
  "Ivy.Json": lazyWithRetry(() => import("@/widgets/primitives/JsonWidget")),
  "Ivy.Html": lazyWithRetry(() =>
    import("@/widgets/primitives/HtmlWidget").then((m) => ({
      default: m.HtmlWidget,
    })),
  ),
  "Ivy.Xml": lazyWithRetry(() => import("@/widgets/primitives/XmlWidget")),
  "Ivy.Error": lazyWithRetry(() =>
    import("@/widgets/primitives/ErrorWidget").then((m) => ({
      default: m.ErrorWidget,
    })),
  ),
  "Ivy.Svg": lazyWithRetry(() =>
    import("@/widgets/primitives/SvgWidget").then((m) => ({
      default: m.SvgWidget,
    })),
  ),
  "Ivy.Image": lazyWithRetry(() =>
    import("@/widgets/primitives/ImageWidget").then((m) => ({
      default: m.ImageWidget,
    })),
  ),
  "Ivy.Iframe": lazyWithRetry(() =>
    import("@/widgets/primitives/IframeWidget").then((m) => ({
      default: m.IframeWidget,
    })),
  ),
  "Ivy.CodeBlock": lazyWithRetry(() => import("@/widgets/primitives/CodeBlockWidget")),
  "Ivy.Fragment": lazyWithRetry(() =>
    import("@/widgets/primitives/FragmentWidget").then((m) => ({
      default: m.FragmentWidget,
    })),
  ),
  "Ivy.Separator": lazyWithRetry(() =>
    import("@/widgets/primitives/SeparatorWidget").then((m) => ({
      default: m.SeparatorWidget,
    })),
  ),
  "Ivy.Skeleton": lazyWithRetry(() =>
    import("@/widgets/primitives/SkeletonWidget").then((m) => ({
      default: m.SkeletonWidget,
    })),
  ),
  "Ivy.Icon": lazyWithRetry(() =>
    import("@/widgets/primitives/IconWidget").then((m) => ({
      default: m.IconWidget,
    })),
  ),
  "Ivy.Box": lazyWithRetry(() =>
    import("@/widgets/primitives/BoxWidget").then((m) => ({
      default: m.BoxWidget,
    })),
  ),
  "Ivy.Embed": lazyWithRetry(() => import("@/widgets/primitives/EmbedWidget")),
  "Ivy.Script": lazyWithRetry(() => import("@/widgets/primitives/ScriptWidget")),
  "Ivy.Callout": lazyWithRetry(() =>
    import("@/widgets/primitives/CalloutWidget").then((m) => ({
      default: m.CalloutWidget,
    })),
  ),
  "Ivy.Kbd": lazyWithRetry(() =>
    import("@/widgets/primitives/KbdWidget").then((m) => ({
      default: m.KbdWidget,
    })),
  ),
  "Ivy.Empty": lazyWithRetry(() =>
    import("@/widgets/primitives/EmptyWidget").then((m) => ({
      default: m.EmptyWidget,
    })),
  ),
  "Ivy.Avatar": lazyWithRetry(() =>
    import("@/widgets/primitives/AvatarWidget").then((m) => ({
      default: m.AvatarWidget,
    })),
  ),
  "Ivy.IvyLogo": lazyWithRetry(() =>
    import("@/widgets/primitives/IvyLogoWidget").then((m) => ({
      default: m.IvyLogoWidget,
    })),
  ),
  "Ivy.Spacer": lazyWithRetry(() =>
    import("@/widgets/primitives/SpacerWidget").then((m) => ({
      default: m.SpacerWidget,
    })),
  ),
  "Ivy.Loading": lazyWithRetry(() =>
    import("@/widgets/primitives/LoadingWidget").then((m) => ({
      default: m.LoadingWidget,
    })),
  ),
  "Ivy.AppHost": AppHostWidget,
  "Ivy.AutoScroll": lazyWithRetry(() =>
    import("@/widgets/primitives/AutoScrollWidget").then((m) => ({
      default: m.AutoScrollWidget,
    })),
  ),
  "Ivy.AudioPlayer": lazyWithRetry(() =>
    import("@/widgets/primitives/AudioPlayerWidget").then((m) => ({
      default: m.AudioPlayerWidget,
    })),
  ),
  "Ivy.VideoPlayer": lazyWithRetry(() =>
    import("@/widgets/primitives/VideoPlayerWidget").then((m) => ({
      default: m.VideoPlayerWidget,
    })),
  ),
  "Ivy.Stepper": lazyWithRetry(() => import("@/widgets/primitives/StepperWidget")),
  "Ivy.Terminal": lazyWithRetry(() => import("@/widgets/primitives/TerminalWidget")),

  // Widgets
  "Ivy.Article": lazyWithRetry(() =>
    import("@/widgets/article/ArticleWidget").then((m) => ({
      default: m.ArticleWidget,
    })),
  ),
  "Ivy.Button": lazyWithRetry(() =>
    import("@/widgets/button/ButtonWidget").then((m) => ({
      default: m.ButtonWidget,
    })),
  ),
  "Ivy.Progress": lazyWithRetry(() =>
    import("@/widgets/progress/ProgressWidget").then((m) => ({
      default: m.ProgressWidget,
    })),
  ),
  "Ivy.StackedProgress": lazyWithRetry(() =>
    import("@/widgets/stackedProgress/StackedProgressWidget").then((m) => ({
      default: m.StackedProgressWidget,
    })),
  ),
  "Ivy.Tooltip": lazyWithRetry(() =>
    import("@/widgets/tooltip/TooltipWidget").then((m) => ({
      default: m.TooltipWidget,
    })),
  ),
  "Ivy.Toolbar": lazyWithRetry(() =>
    import("@/widgets/toolbar/ToolbarWidget").then((m) => ({
      default: m.ToolbarWidget,
    })),
  ),
  "Ivy.Slot": lazyWithRetry(() =>
    import("@/widgets/slot/SlotWidget").then((m) => ({
      default: m.SlotWidget,
    })),
  ),
  "Ivy.Card": lazyWithRetry(() =>
    import("@/widgets/card/CardWidget").then((m) => ({
      default: m.CardWidget,
    })),
  ),
  "Ivy.Sheet": lazyWithRetry(() =>
    import("@/widgets/sheet/SheetWidget").then((m) => ({
      default: m.SheetWidget,
    })),
  ),
  "Ivy.Badge": lazyWithRetry(() =>
    import("@/widgets/badge/BadgeWidget").then((m) => ({
      default: m.BadgeWidget,
    })),
  ),
  "Ivy.Breadcrumbs": lazyWithRetry(() =>
    import("@/widgets/breadcrumbs/BreadcrumbsWidget").then((m) => ({
      default: m.BreadcrumbsWidget,
    })),
  ),
  "Ivy.Expandable": lazyWithRetry(() =>
    import("@/widgets/expandable/ExpandableWidget").then((m) => ({
      default: m.ExpandableWidget,
    })),
  ),
  "Ivy.Chat": lazyWithRetry(() =>
    import("@/widgets/chat/ChatWidget").then((m) => ({
      default: m.ChatWidget,
    })),
  ),
  "Ivy.ChatMessage": lazyWithRetry(() =>
    import("@/widgets/chat/ChatMessageWidget").then((m) => ({
      default: m.ChatMessageWidget,
    })),
  ),
  "Ivy.ChatLoading": lazyWithRetry(() =>
    import("@/widgets/chat/ChatLoadingWidget").then((m) => ({
      default: m.ChatLoadingWidget,
    })),
  ),
  "Ivy.ChatStatus": lazyWithRetry(() =>
    import("@/widgets/chat/ChatStatusWidget").then((m) => ({
      default: m.ChatStatusWidget,
    })),
  ),
  "Ivy.DropDownMenu": lazyWithRetry(() =>
    import("@/widgets/dropDownMenu/DropDownMenuWidget").then((m) => ({
      default: m.DropDownMenuWidget,
    })),
  ),
  "Ivy.Pagination": lazyWithRetry(() =>
    import("@/widgets/pagination/PaginationWidget").then((m) => ({
      default: m.PaginationWidget,
    })),
  ),
  "Ivy.Kanban": lazyWithRetry(() =>
    import("@/widgets/kanban/KanbanWidget").then((m) => ({
      default: m.KanbanWidget,
    })),
  ),
  "Ivy.KanbanCard": lazyWithRetry(() =>
    import("@/widgets/kanban/KanbanCardWidget").then((m) => ({
      default: m.KanbanCardWidget,
    })),
  ),
  "Ivy.Calendar": lazyWithRetry(() =>
    import("@/widgets/calendar/CalendarWidget").then((m) => ({
      default: m.CalendarWidget,
    })),
  ),
  "Ivy.CalendarEvent": lazyWithRetry(() =>
    import("@/widgets/calendar/CalendarEventWidget").then((m) => ({
      default: m.CalendarEventWidget,
    })),
  ),

  // Layouts
  "Ivy.StackLayout": lazyWithRetry(() =>
    import("@/widgets/layouts/StackLayoutWidget").then((m) => ({
      default: m.StackLayoutWidget,
    })),
  ),
  "Ivy.GridLayout": lazyWithRetry(() =>
    import("@/widgets/layouts/GridLayoutWidget").then((m) => ({
      default: m.GridLayoutWidget,
    })),
  ),
  "Ivy.HeaderLayout": lazyWithRetry(() =>
    import("@/widgets/layouts/HeaderLayoutWidget").then((m) => ({
      default: m.HeaderLayoutWidget,
    })),
  ),
  "Ivy.FooterLayout": lazyWithRetry(() =>
    import("@/widgets/layouts/FooterLayoutWidget").then((m) => ({
      default: m.FooterLayoutWidget,
    })),
  ),
  "Ivy.TabsLayout": lazyWithRetry(() =>
    import("@/widgets/layouts/tabs/TabsLayoutWidget").then((m) => ({
      default: m.TabsLayoutWidget,
    })),
  ),
  "Ivy.Tab": lazyWithRetry(() =>
    import("@/widgets/layouts/tabs/TabWidget").then((m) => ({
      default: m.TabWidget,
    })),
  ),
  "Ivy.SidebarLayout": lazyWithRetry(() =>
    import("@/widgets/layouts/sidebar/SidebarLayoutWidget").then((m) => ({
      default: m.SidebarLayoutWidget,
    })),
  ),
  "Ivy.SidebarMenu": lazyWithRetry(() =>
    import("@/widgets/layouts/sidebar/SidebarLayoutWidget").then((m) => ({
      default: m.SidebarMenuWidget,
    })),
  ),
  "Ivy.ResizablePanelGroup": lazyWithRetry(() =>
    import("@/widgets/layouts/ResizablePanelGroupWidget").then((m) => ({
      default: m.ResizablePanelGroupWidget,
    })),
  ),
  "Ivy.ResizablePanel": lazyWithRetry(() =>
    import("@/widgets/layouts/ResizablePanelGroupWidget").then((m) => ({
      default: m.ResizablePanelWidget,
    })),
  ),
  "Ivy.FloatingPanel": lazyWithRetry(() =>
    import("@/widgets/layouts/FloatingPanelWidget").then((m) => ({
      default: m.FloatingPanelWidget,
    })),
  ),

  // Inputs
  "Ivy.Field": lazyWithRetry(() =>
    import("@/widgets/inputs/FieldWidget").then((m) => ({
      default: m.FieldWidget,
    })),
  ),
  "Ivy.TextInput": lazyWithRetry(() =>
    import("@/widgets/inputs/TextInputWidget/TextInputWidget").then((m) => ({
      default: m.TextInputWidget,
    })),
  ),
  "Ivy.BoolInput": lazyWithRetry(() =>
    import("@/widgets/inputs/BoolInputWidget").then((m) => ({
      default: m.BoolInputWidget,
    })),
  ),
  "Ivy.DateTimeInput": lazyWithRetry(() =>
    import("@/widgets/inputs/DateTimeInputWidget/DateTimeInputWidget").then((m) => ({
      default: m.DateTimeInputWidget,
    })),
  ),
  "Ivy.NumberInput": lazyWithRetry(() =>
    import("@/widgets/inputs/NumberInputWidget").then((m) => ({
      default: m.NumberInputWidget,
    })),
  ),
  "Ivy.NumberRangeInput": lazyWithRetry(() =>
    import("@/widgets/inputs/NumberRangeInputWidget").then((m) => ({
      default: m.NumberRangeInputWidget,
    })),
  ),
  "Ivy.SelectInput": lazyWithRetry(() =>
    import("@/widgets/inputs/SelectInputWidget").then((m) => ({
      default: m.SelectInputWidget,
    })),
  ),
  "Ivy.ReadOnlyInput": lazyWithRetry(() =>
    import("@/widgets/inputs/ReadOnlyInputWidget").then((m) => ({
      default: m.ReadOnlyInputWidget,
    })),
  ),
  "Ivy.ColorInput": lazyWithRetry(() =>
    import("@/widgets/inputs/ColorInputWidget").then((m) => ({
      default: m.ColorInputWidget,
    })),
  ),
  "Ivy.IconInput": lazyWithRetry(() =>
    import("@/widgets/inputs/IconInputWidget").then((m) => ({
      default: m.IconInputWidget,
    })),
  ),
  "Ivy.FeedbackInput": lazyWithRetry(() =>
    import("@/widgets/inputs/FeedbackInputWidget").then((m) => ({
      default: m.FeedbackInputWidget,
    })),
  ),
  "Ivy.AsyncSelectInput": lazyWithRetry(() =>
    import("@/widgets/inputs/AsyncSelectInputWidget").then((m) => ({
      default: m.AsyncSelectInputWidget,
    })),
  ),
  "Ivy.DateRangeInput": lazyWithRetry(() =>
    import("@/widgets/inputs/DateRangeInputWidget").then((m) => ({
      default: m.DateRangeInputWidget,
    })),
  ),
  "Ivy.FileInput": lazyWithRetry(() =>
    import("@/widgets/inputs/FileInputWidget").then((m) => ({
      default: m.FileInputWidget,
    })),
  ),
  "Ivy.ContentInput": lazyWithRetry(() =>
    import("@/widgets/inputs/ContentInputWidget/ContentInputWidget").then((m) => ({
      default: m.ContentInputWidget,
    })),
  ),
  "Ivy.SignatureInput": lazyWithRetry(() =>
    import("@/widgets/inputs/SignatureInputWidget").then((m) => ({
      default: m.SignatureInputWidget,
    })),
  ),

  "Ivy.CodeInput": lazyWithRetry(() => import("@/widgets/inputs/code/CodeInputWidget")),
  "Ivy.AudioInput": lazyWithRetry(() => import("@/widgets/inputs/AudioInputWidget")),
  "Ivy.CameraInput": lazyWithRetry(() => import("@/widgets/cameraInput/CameraInputWidget")),

  // Forms
  "Ivy.Form": lazyWithRetry(() =>
    import("@/widgets/forms/FormWidget").then((m) => ({
      default: m.FormWidget,
    })),
  ),

  // File Pickers
  "Ivy.FileDialog": lazyWithRetry(() =>
    import("@/widgets/filePicker/FileDialogWidget").then((m) => ({
      default: m.FileDialogWidget,
    })),
  ),
  "Ivy.SaveDialog": lazyWithRetry(() =>
    import("@/widgets/filePicker/SaveDialogWidget").then((m) => ({
      default: m.SaveDialogWidget,
    })),
  ),
  "Ivy.FolderDialog": lazyWithRetry(() =>
    import("@/widgets/filePicker/FolderDialogWidget").then((m) => ({
      default: m.FolderDialogWidget,
    })),
  ),

  // Dialogs
  "Ivy.Dialog": lazyWithRetry(() =>
    import("@/widgets/dialogs/DialogWidget").then((m) => ({
      default: m.DialogWidget,
    })),
  ),
  "Ivy.DialogHeader": lazyWithRetry(() =>
    import("@/widgets/dialogs/DialogHeaderWidget").then((m) => ({
      default: m.DialogHeaderWidget,
    })),
  ),
  "Ivy.DialogBody": lazyWithRetry(() =>
    import("@/widgets/dialogs/DialogBodyWidget").then((m) => ({
      default: m.DialogBodyWidget,
    })),
  ),
  "Ivy.DialogFooter": lazyWithRetry(() =>
    import("@/widgets/dialogs/DialogFooterWidget").then((m) => ({
      default: m.DialogFooterWidget,
    })),
  ),

  // Blades
  "Ivy.BladeContainer": lazyWithRetry(() =>
    import("@/widgets/blades/BladeContainerWidget").then((m) => ({
      default: m.BladeContainerWidget,
    })),
  ),
  "Ivy.Blade": lazyWithRetry(() =>
    import("@/widgets/blades/BladeWidget").then((m) => ({
      default: m.BladeWidget,
    })),
  ),

  // Tables
  "Ivy.Table": lazyWithRetry(() =>
    import("@/widgets/tables/TableWidget").then((m) => ({
      default: m.TableWidget,
    })),
  ),
  "Ivy.TableRow": lazyWithRetry(() =>
    import("@/widgets/tables/TableRowWidget").then((m) => ({
      default: m.TableRowWidget,
    })),
  ),
  "Ivy.TableCell": lazyWithRetry(() =>
    import("@/widgets/tables/TableCellWidget").then((m) => ({
      default: m.TableCellWidget,
    })),
  ),

  // DataTables
  "Ivy.DataTable": lazyWithRetry(() => import("@/widgets/dataTables/DataTableWidget")),

  // Lists
  "Ivy.List": lazyWithRetry(() =>
    import("@/widgets/lists/ListWidget").then((m) => ({
      default: m.ListWidget,
    })),
  ),
  "Ivy.ListItem": lazyWithRetry(() =>
    import("@/widgets/lists/ListItemWidget").then((m) => ({
      default: m.ListItemWidget,
    })),
  ),

  // Tree
  "Ivy.Tree": lazyWithRetry(() =>
    import("@/widgets/tree/TreeWidget").then((m) => ({
      default: m.TreeWidget,
    })),
  ),

  // Details
  "Ivy.Details": lazyWithRetry(() =>
    import("@/widgets/details/DetailsWidget").then((m) => ({
      default: m.DetailsWidget,
    })),
  ),
  "Ivy.Detail": lazyWithRetry(() =>
    import("@/widgets/details/DetailWidget").then((m) => ({
      default: m.DetailWidget,
    })),
  ),

  // Charts
  "Ivy.LineChart": lazyWithRetry(() => import("@/widgets/charts/LineChartWidget")),
  "Ivy.PieChart": lazyWithRetry(() => import("@/widgets/charts/PieChartWidget")),
  "Ivy.AreaChart": lazyWithRetry(() => import("@/widgets/charts/AreaChartWidget")),
  "Ivy.BarChart": lazyWithRetry(() => import("@/widgets/charts/BarChartWidget")),
  "Ivy.ScatterChart": lazyWithRetry(() => import("@/widgets/charts/ScatterChartWidget")),
  "Ivy.RadarChart": lazyWithRetry(() => import("@/widgets/charts/RadarChartWidget")),
  "Ivy.SankeyChart": lazyWithRetry(() => import("@/widgets/charts/SankeyChartWidget")),
  "Ivy.ChordChart": lazyWithRetry(() => import("@/widgets/charts/ChordChartWidget")),
  "Ivy.FunnelChart": lazyWithRetry(() => import("@/widgets/charts/FunnelChartWidget")),
  "Ivy.GaugeChart": lazyWithRetry(() => import("@/widgets/charts/GaugeChartWidget")),

  // Effects
  "Ivy.Confetti": lazyWithRetry(() => import("@/widgets/effects/ConfettiWidget")),
  "Ivy.Animation": lazyWithRetry(() => import("@/widgets/effects/AnimationWidget")),

  // Internal
  "Ivy.Docs.Shared.Internal.SmartSearch": lazyWithRetry(() =>
    import("@/docs-internal/SmartSearch").then((m) => ({
      default: m.SmartSearch,
    })),
  ),
  "Ivy.Widgets.Internal.SidebarNews": lazyWithRetry(
    () => import("@/widgets/internal/SidebarNewsWidget"),
  ),
  "Ivy.Widgets.Internal.ThemeColorPicker": lazyWithRetry(() =>
    import("@/widgets/internal/ThemeColorPickerWidget").then((m) => ({
      default: m.ThemeColorPickerWidget,
    })),
  ),
};
