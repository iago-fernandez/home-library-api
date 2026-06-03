CREATE OR REPLACE FUNCTION set_catalog_number_on_move() RETURNS TRIGGER AS $$
BEGIN
  -- Si el library_id ha cambiado (se ha movido de librería)
  IF NEW.library_id IS DISTINCT FROM OLD.library_id THEN
      -- Bloquear la librería de destino para evitar condiciones de carrera
      PERFORM 1 FROM libraries WHERE id = NEW.library_id FOR UPDATE;
      
      -- Obtener el número de catálogo máximo actual en la librería de destino y sumar 1
      SELECT COALESCE(MAX(catalog_number), 0) + 1 INTO NEW.catalog_number 
      FROM books 
      WHERE library_id = NEW.library_id;
  END IF;
  
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_set_catalog_number_on_move
BEFORE UPDATE OF library_id ON books
FOR EACH ROW
EXECUTE FUNCTION set_catalog_number_on_move();
