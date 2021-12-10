import { derived } from "svelte/store";
import { ScreenWidth, screenWidth } from "./screenDimensions";

export const iconSize = derived(screenWidth, ($screenWidth) => {
    return $screenWidth === ScreenWidth.ExtraSmall ? "1.2em" : "1.5em";
});
