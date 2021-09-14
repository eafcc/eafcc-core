enum eafcc_UpdateNotifyLevel {
  NoNotify,
  NotifyWithoutChangedKeysByGlobal,
  NotifyWithoutChangedKeysInNamespace,
  NotifyWithMaybeChangedKeys,
};
typedef uint32_t eafcc_UpdateNotifyLevel;

enum eafcc_ViewMode {
  OverlaidView,
  AllLinkedResView,
};
typedef uint32_t eafcc_ViewMode;

typedef struct eafcc_CFGCenter eafcc_CFGCenter;

typedef struct eafcc_Differ eafcc_Differ;

typedef struct eafcc_NamespaceScopedCFGCenter eafcc_NamespaceScopedCFGCenter;

typedef struct eafcc_WhoAmI eafcc_WhoAmI;

typedef struct {
  float pri;
  bool is_neg;
  char *link_path;
  char *rule_path;
  char *res_path;
} eafcc_ConfigValueReason;

typedef struct {
  char *key;
  char *content_type;
  char *value;
  eafcc_ConfigValueReason *reason;
} eafcc_ConfigValue;

const eafcc_CFGCenter *new_config_center_client(const char *cfg);

void free_config_center(eafcc_CFGCenter *cc);

const eafcc_NamespaceScopedCFGCenter *create_namespace(const eafcc_CFGCenter *cc,
                                                       const char *namespace_,
                                                       eafcc_UpdateNotifyLevel notify_level,
                                                       void (*cb)(const eafcc_Differ *differ, const void *usre_data),
                                                       const void *user_data);

void free_namespace(const eafcc_NamespaceScopedCFGCenter *ns);

const eafcc_WhoAmI *new_context(const char *val);

void free_context(eafcc_WhoAmI *ctx);

eafcc_ConfigValue *get_config(const eafcc_NamespaceScopedCFGCenter *ns,
                              const eafcc_WhoAmI *whoami,
                              char **keys,
                              uintptr_t key_cnt,
                              eafcc_ViewMode view_mode,
                              uint8_t need_explain);

void free_config_value(eafcc_ConfigValue *v, uintptr_t n);

eafcc_ConfigValue *differ_get_from_old(const eafcc_Differ *differ,
                                       const eafcc_WhoAmI *whoami,
                                       char **keys,
                                       uintptr_t key_cnt,
                                       eafcc_ViewMode view_mode,
                                       uint8_t need_explain);

eafcc_ConfigValue *differ_get_from_new(const eafcc_Differ *differ,
                                       const eafcc_WhoAmI *whoami,
                                       char **keys,
                                       uintptr_t key_cnt,
                                       eafcc_ViewMode view_mode,
                                       uint8_t need_explain);
