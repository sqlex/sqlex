package me.danwi.sqlex.common;

import java.util.List;
import java.util.Objects;

public class StringUtils {
    public static class ReplaceInfo {
        private int start;
        private int end;
        private String content;

        public ReplaceInfo(int start, int end, String content) {
            this.start = start;
            this.end = end;
            this.content = content;
        }

        @Override
        public boolean equals(Object o) {
            if (this == o) return true;
            if (o == null || getClass() != o.getClass()) return false;
            ReplaceInfo that = (ReplaceInfo) o;
            return start == that.start && end == that.end && Objects.equals(content, that.content);
        }

        @Override
        public int hashCode() {
            return Objects.hash(start, end, content);
        }
    }

    /**
     * 根据传入的替换信息来替换字符串的部分内容
     *
     * @param text     原来的字符串
     * @param replaces 替换信息
     * @return 替换后的字符串
     */
    public static String replace(String text, List<ReplaceInfo> replaces) {
        StringBuilder result = new StringBuilder(text);
        for (ReplaceInfo replace : replaces) {
            //替换字符串内容
            result.replace(replace.start, replace.end, replace.content);
            //计算尺寸的增长
            int sizeGrow = replace.content.length() - (replace.end - replace.start);
            //由于变成了原来的字符串,现在需要重新计算接下来的位置
            replaces.forEach(r -> {
                if (r.start >= replace.end) {
                    r.start += sizeGrow;
                    r.end += sizeGrow;
                }
            });
        }
        return result.toString();
    }
}
