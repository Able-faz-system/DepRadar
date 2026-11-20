import { useTranslation } from "react-i18next";

export function LanguageSelector() {
  const { i18n } = useTranslation();

  return (
    <div className="lang-selector" style={{ display: "inline-flex", alignItems: "center" }}>
      <select
        value={i18n.resolvedLanguage || "en"}
        onChange={(e) => i18n.changeLanguage(e.target.value)}
        aria-label="Select language"
        className="btn btn-ghost mono"
        style={{ padding: "0.25rem 0.5rem", fontSize: "0.85rem", cursor: "pointer" }}
      >
        <option value="en">EN</option>
      </select>
    </div>
  );
}

export default LanguageSelector;
