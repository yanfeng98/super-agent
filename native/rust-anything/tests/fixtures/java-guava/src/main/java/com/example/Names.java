package com.example;

import com.google.common.base.Strings;
import org.apache.commons.lang3.StringUtils;

public class Names {
    public static String pad(String s) {
        return Strings.padStart(StringUtils.trim(s), 8, ' ');
    }
}
