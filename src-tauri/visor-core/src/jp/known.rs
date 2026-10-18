//! Palabras conocidas (globales, no por juego): formas de diccionario. Se guardan en la tabla
//! `known_words` de la base (antes `known-words.json`); aqui esta la copia en memoria.
use std::collections::BTreeSet;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Known {
    set: BTreeSet<String>,
}

impl Known {
    /// Construye el conjunto a partir de una lista (descarta las vacias).
    pub fn from_list(list: Vec<String>) -> Known {
        Known { set: list.into_iter().filter(|w| !w.trim().is_empty()).collect() }
    }

    pub fn contains(&self, base: &str) -> bool {
        self.set.contains(base)
    }

    /// Alterna la palabra; devuelve `true` si queda como conocida.
    pub fn toggle(&mut self, base: &str) -> bool {
        let base = base.trim();
        if base.is_empty() {
            return false;
        }
        if self.set.remove(base) {
            false
        } else {
            self.set.insert(base.to_string());
            true
        }
    }

    pub fn list(&self) -> Vec<String> {
        self.set.iter().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.set.len()
    }

    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alternar_guardar_y_cargar() {
        let db = crate::db::Db::open_memory().unwrap();
        assert!(Known::from_list(db.with(crate::db::data::known_load).unwrap()).is_empty());
        let mut k = Known::default();
        assert!(k.toggle("雪"));
        assert!(k.toggle("降る"));
        assert!(!k.toggle("雪"));
        assert!(!k.toggle("  "));
        // La base recibe cada cambio por separado.
        db.with(|c| crate::db::data::known_set(c, "雪", true)).unwrap();
        db.with(|c| crate::db::data::known_set(c, "降る", true)).unwrap();
        db.with(|c| crate::db::data::known_set(c, "雪", false)).unwrap();
        db.with(|c| crate::db::data::known_set(c, "  ", true)).unwrap();
        let k2 = Known::from_list(db.with(crate::db::data::known_load).unwrap());
        assert!(k2.contains("降る") && !k2.contains("雪"));
        assert_eq!(k2.len(), 1);
        assert_eq!(k2, k);
    }
}
